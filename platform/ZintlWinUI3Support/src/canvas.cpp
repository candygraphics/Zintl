#include "common.h"

#include <d2d1_1.h>
#include <d2d1_1helper.h>
#include <d3d11.h>
#include <dwrite.h>
#include <dxgi1_3.h>
#include <microsoft.ui.xaml.media.dxinterop.h>

#include <algorithm>
#include <cmath>
#include <cwctype>
#include <iterator>
#include <map>
#include <tuple>
#include <unordered_map>
#include <vector>

#include <winrt/Microsoft.UI.Input.h>
#include <winrt/Microsoft.UI.Xaml.Media.h>
#include <winrt/Windows.System.h>

using namespace zpd;
using Windows::Foundation::IInspectable;
namespace xinput = winrt::Microsoft::UI::Xaml::Input;
namespace uinput = winrt::Microsoft::UI::Input;
using winrt::Microsoft::UI::Dispatching::DispatcherQueue;

namespace {

/* Wheel distance in DIPs for one notch (120 units). */
constexpr double WHEEL_NOTCH = 48.0;

ID2D1Factory1* d2d_factory() {
  // Released by the process; factories outlive every canvas.
  static ID2D1Factory1* factory = [] {
    com_ptr<ID2D1Factory1> value;
    D2D1_FACTORY_OPTIONS options{};
    check_hresult(D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, __uuidof(ID2D1Factory1), &options, value.put_void()));
    return value.detach();
  }();
  return factory;
}

IDWriteFactory* dwrite_factory() {
  static IDWriteFactory* factory = [] {
    com_ptr<IDWriteFactory> value;
    check_hresult(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory), reinterpret_cast<IUnknown**>(value.put())));
    return value.detach();
  }();
  return factory;
}

double seconds() {
  static LARGE_INTEGER frequency = [] {
    LARGE_INTEGER value;
    QueryPerformanceFrequency(&value);
    return value;
  }();
  LARGE_INTEGER now;
  QueryPerformanceCounter(&now);
  return static_cast<double>(now.QuadPart) / static_cast<double>(frequency.QuadPart);
}

D2D1_COLOR_F color_f(zpd_winui3_color color) {
  return {static_cast<float>(color.red), static_cast<float>(color.green), static_cast<float>(color.blue), static_cast<float>(color.alpha)};
}

D2D1_POINT_2F point(double x, double y) { return {static_cast<float>(x), static_cast<float>(y)}; }

/* Text formats and metrics for one font description. */
struct FontKey {
  std::wstring family;
  double size;
  int32_t weight;
  bool italic;

  bool operator<(FontKey const& other) const {
    return std::tie(family, size, weight, italic) < std::tie(other.family, other.size, other.weight, other.italic);
  }

  static FontKey from(zpd_winui3_font const& font) {
    auto family = std::wstring(string_from(font.family));
    if (family.empty()) family = L"Segoe UI";
    return {family, font.size > 0 ? font.size : 14.0, font.weight > 0 ? font.weight : 400, font.italic};
  }
};

struct FontMetrics {
  double ascent;
  double descent;
  double leading;
};

struct FontCache {
  std::map<FontKey, com_ptr<IDWriteTextFormat>> formats;
  std::map<FontKey, FontMetrics> metrics;

  IDWriteTextFormat* format(FontKey const& key) {
    auto& slot = formats[key];
    if (!slot) {
      check_hresult(dwrite_factory()->CreateTextFormat(
          key.family.c_str(), nullptr, static_cast<DWRITE_FONT_WEIGHT>(key.weight),
          key.italic ? DWRITE_FONT_STYLE_ITALIC : DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL,
          static_cast<float>(key.size), L"", slot.put()));
      slot->SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
    }
    return slot.get();
  }

  /* Ascent, descent, and line gap of the font, scaled to its size. */
  FontMetrics const& font_metrics(FontKey const& key) {
    auto found = metrics.find(key);
    if (found != metrics.end()) return found->second;
    com_ptr<IDWriteFontCollection> collection;
    check_hresult(dwrite_factory()->GetSystemFontCollection(collection.put()));
    UINT32 index{};
    BOOL exists{};
    check_hresult(collection->FindFamilyName(key.family.c_str(), &index, &exists));
    if (!exists) check_hresult(collection->FindFamilyName(L"Segoe UI", &index, &exists));
    FontMetrics result{key.size * 0.8, key.size * 0.2, 0};
    if (exists) {
      com_ptr<IDWriteFontFamily> family;
      check_hresult(collection->GetFontFamily(index, family.put()));
      com_ptr<IDWriteFont> font;
      check_hresult(family->GetFirstMatchingFont(static_cast<DWRITE_FONT_WEIGHT>(key.weight), DWRITE_FONT_STRETCH_NORMAL,
                                                 key.italic ? DWRITE_FONT_STYLE_ITALIC : DWRITE_FONT_STYLE_NORMAL, font.put()));
      DWRITE_FONT_METRICS native{};
      font->GetMetrics(&native);
      double unit = key.size / native.designUnitsPerEm;
      result = {native.ascent * unit, native.descent * unit, native.lineGap * unit};
    }
    return metrics.emplace(key, result).first->second;
  }

  com_ptr<IDWriteTextLayout> layout(zpd_winui3_string text, FontKey const& key) {
    auto value = string_from(text);
    com_ptr<IDWriteTextLayout> result;
    check_hresult(dwrite_factory()->CreateTextLayout(value.c_str(), value.size(), format(key), 1.0e6f, 1.0e6f, result.put()));
    return result;
  }
};

FontCache& fonts() {
  thread_local FontCache cache;
  return cache;
}

com_ptr<ID2D1PathGeometry> geometry(zpd_winui3_path const& path) {
  com_ptr<ID2D1PathGeometry> result;
  check_hresult(d2d_factory()->CreatePathGeometry(result.put()));
  com_ptr<ID2D1GeometrySink> sink;
  check_hresult(result->Open(sink.put()));
  sink->SetFillMode(path.even_odd ? D2D1_FILL_MODE_ALTERNATE : D2D1_FILL_MODE_WINDING);
  bool open{};
  D2D1_POINT_2F start{}, current{};
  auto begin = [&](D2D1_POINT_2F at) {
    if (open) sink->EndFigure(D2D1_FIGURE_END_OPEN);
    sink->BeginFigure(at, D2D1_FIGURE_BEGIN_FILLED);
    open = true;
    start = current = at;
  };
  for (size_t index = 0; index < path.length; ++index) {
    auto const& element = path.elements[index];
    auto const* p = element.points;
    if (element.verb == ZPD_WINUI3_PATH_MOVE) {
      begin(point(p[0], p[1]));
      continue;
    }
    if (element.verb == ZPD_WINUI3_PATH_CLOSE) {
      if (open) sink->EndFigure(D2D1_FIGURE_END_CLOSED);
      open = false;
      current = start;
      continue;
    }
    // A segment after a close continues from the closed figure's start.
    if (!open) begin(current);
    switch (element.verb) {
      case ZPD_WINUI3_PATH_LINE:
        current = point(p[0], p[1]);
        sink->AddLine(current);
        break;
      case ZPD_WINUI3_PATH_QUAD:
        current = point(p[2], p[3]);
        sink->AddQuadraticBezier({point(p[0], p[1]), current});
        break;
      case ZPD_WINUI3_PATH_CUBIC:
        current = point(p[4], p[5]);
        sink->AddBezier({point(p[0], p[1]), point(p[2], p[3]), current});
        break;
      default: break;
    }
  }
  if (open) sink->EndFigure(D2D1_FIGURE_END_OPEN);
  check_hresult(sink->Close());
  return result;
}

com_ptr<ID2D1StrokeStyle> stroke_style(zpd_winui3_stroke const& stroke) {
  static constexpr D2D1_CAP_STYLE caps[] = {D2D1_CAP_STYLE_FLAT, D2D1_CAP_STYLE_ROUND, D2D1_CAP_STYLE_SQUARE};
  static constexpr D2D1_LINE_JOIN joins[] = {D2D1_LINE_JOIN_MITER_OR_BEVEL, D2D1_LINE_JOIN_ROUND, D2D1_LINE_JOIN_BEVEL};
  auto cap = caps[std::clamp(stroke.cap, 0, 2)];
  // Direct2D measures dashes in stroke widths.
  double width = stroke.width > 0 ? stroke.width : 1.0;
  std::vector<float> dashes;
  for (size_t index = 0; index < stroke.dashes_length; ++index) dashes.push_back(static_cast<float>(stroke.dashes[index] / width));
  D2D1_STROKE_STYLE_PROPERTIES properties{cap, cap, cap, joins[std::clamp(stroke.join, 0, 2)], static_cast<float>(stroke.miter_limit),
                                          dashes.empty() ? D2D1_DASH_STYLE_SOLID : D2D1_DASH_STYLE_CUSTOM,
                                          static_cast<float>(stroke.dash_offset / width)};
  com_ptr<ID2D1StrokeStyle> result;
  check_hresult(d2d_factory()->CreateStrokeStyle(properties, dashes.empty() ? nullptr : dashes.data(), static_cast<UINT32>(dashes.size()), result.put()));
  return result;
}

/* Hands a swap chain to the panel after scaling it to DIPs. */
struct SwapChainPanelProxy : implements<SwapChainPanelProxy, ISwapChainPanelNative> {
  com_ptr<ISwapChainPanelNative> panel;
  com_ptr<IDXGISwapChain> swap_chain;
  float scale_x{1};
  float scale_y{1};

  void apply_scale() const {
    if (!swap_chain) return;
    if (auto scaled = swap_chain.try_as<IDXGISwapChain2>()) {
      DXGI_MATRIX_3X2_F inverse{1.0f / scale_x, 0, 0, 1.0f / scale_y, 0, 0};
      scaled->SetMatrixTransform(&inverse);
    }
  }

  HRESULT __stdcall SetSwapChain(IDXGISwapChain* value) noexcept override {
    swap_chain.copy_from(value);
    apply_scale();
    return panel->SetSwapChain(value);
  }
};

/* Saved drawing state; `layers` counts clip layers pushed at this level. */
struct GraphicsState {
  D2D1_MATRIX_3X2_F transform{D2D1::Matrix3x2F::Identity()};
  float alpha{1};
  bool antialias{true};
  int layers{};
};

struct CanvasState;

}  // namespace

struct zpd_winui3_draw {
  CanvasState* canvas;
  ID2D1DeviceContext* context;
  GraphicsState state;
  std::vector<GraphicsState> stack;

  void restore();
  void finish();
  com_ptr<ID2D1Brush> brush(zpd_winui3_paint const& paint);
};

namespace {

struct CanvasState : std::enable_shared_from_this<CanvasState> {
  int32_t mode{};
  SwapChainPanel panel{nullptr};
  /* Hosts the panel and takes input and focus, so the whole area is hit-testable. */
  Grid host{nullptr};
  Callback callback;
  zpd_winui3_canvas_callbacks callbacks{};
  bool continuous{};
  double fps{};
  bool loaded{};
  bool draw_scheduled{};
  bool drawing{};
  event_token rendering{};
  double width{};
  double height{};
  float scale_x{1};
  float scale_y{1};
  double last_time{};
  com_ptr<SwapChainPanelProxy> proxy;
  com_ptr<ISwapChainPanelNative> proxy_native;

  /* Direct2D resources; dropped together when the device is lost. */
  com_ptr<ID3D11Device> d3d;
  com_ptr<ID2D1Device> device;
  com_ptr<ID2D1DeviceContext> context;
  com_ptr<IDXGISwapChain1> swap_chain;
  com_ptr<ID2D1Bitmap1> target;
  com_ptr<ID2D1SolidColorBrush> solid;
  UINT pixel_width{};
  UINT pixel_height{};
  float target_scale_x{};
  float target_scale_y{};
  /* Images by id, with the frame that last drew them. */
  std::unordered_map<uint64_t, std::pair<com_ptr<ID2D1Bitmap1>, uint64_t>> images;
  uint64_t frame{};

  /* Input state. */
  int32_t pressed_button{-1};
  uint32_t click_count{};
  int32_t last_click_button{-1};
  double last_click_time{};
  Windows::Foundation::Point last_click_position{};
  std::wstring pending_text;
  wchar_t high_surrogate{};
  bool reported_focus{};

  ~CanvasState() { stop_rendering(); }

  bool has_callback() const { return callback.data != nullptr; }

  UINT pixels(double dips, float scale) const { return std::max<UINT>(1, static_cast<UINT>(std::lround(dips * scale))); }

  double title_bar_inset() const { return zpd::title_bar_inset(host); }

  void request_draw() {
    if (draw_scheduled || (continuous && rendering)) return;
    draw_scheduled = true;
    std::weak_ptr<CanvasState> weak = shared_from_this();
    DispatcherQueue::GetForCurrentThread().TryEnqueue([weak] {
      if (auto state = weak.lock()) {
        state->draw_scheduled = false;
        state->draw();
      }
    });
  }

  void update_rendering() {
    bool wanted = continuous && loaded;
    if (wanted && !rendering) {
      std::weak_ptr<CanvasState> weak = shared_from_this();
      rendering = Media::CompositionTarget::Rendering([weak](IInspectable const&, IInspectable const&) {
        auto state = weak.lock();
        if (!state) return;
        // A capped rate skips refreshes that come early; 2 ms absorbs timer jitter.
        if (state->fps > 0 && state->last_time > 0 && seconds() - state->last_time < 1.0 / state->fps - 0.002) return;
        state->draw();
      });
    } else if (!wanted) {
      stop_rendering();
    }
  }

  void stop_rendering() {
    if (!rendering) return;
    try {
      Media::CompositionTarget::Rendering(rendering);
    } catch (...) {
    }
    rendering = {};
  }

  void lose_device() {
    if (context) context->SetTarget(nullptr);
    if (swap_chain) {
      try {
        panel.as<ISwapChainPanelNative>()->SetSwapChain(nullptr);
      } catch (...) {
      }
    }
    images.clear();
    target = nullptr;
    solid = nullptr;
    swap_chain = nullptr;
    context = nullptr;
    device = nullptr;
    d3d = nullptr;
  }

  void apply_scale() const {
    if (auto scaled = swap_chain.try_as<IDXGISwapChain2>()) {
      DXGI_MATRIX_3X2_F inverse{1.0f / scale_x, 0, 0, 1.0f / scale_y, 0, 0};
      check_hresult(scaled->SetMatrixTransform(&inverse));
    }
  }

  void create_target() {
    com_ptr<IDXGISurface> surface;
    check_hresult(swap_chain->GetBuffer(0, IID_PPV_ARGS(surface.put())));
    auto properties = D2D1::BitmapProperties1(D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                                              D2D1::PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM, D2D1_ALPHA_MODE_PREMULTIPLIED),
                                              96.0f * scale_x, 96.0f * scale_y);
    check_hresult(context->CreateBitmapFromDxgiSurface(surface.get(), &properties, target.put()));
    context->SetTarget(target.get());
    // Drawing coordinates stay in DIPs at any scale.
    context->SetDpi(96.0f * scale_x, 96.0f * scale_y);
    target_scale_x = scale_x;
    target_scale_y = scale_y;
  }

  void create_device() {
    UINT flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
    D3D_FEATURE_LEVEL levels[] = {D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_10_0};
    HRESULT result = D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr, flags, levels, ARRAYSIZE(levels), D3D11_SDK_VERSION, d3d.put(), nullptr, nullptr);
    // Without a usable GPU, the WARP rasterizer draws in software.
    if (FAILED(result)) check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_WARP, nullptr, flags, levels, ARRAYSIZE(levels), D3D11_SDK_VERSION, d3d.put(), nullptr, nullptr));
    auto dxgi = d3d.as<IDXGIDevice>();
    check_hresult(d2d_factory()->CreateDevice(dxgi.get(), device.put()));
    check_hresult(device->CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE, context.put()));
    check_hresult(context->CreateSolidColorBrush(D2D1::ColorF(0, 0, 0, 1), solid.put()));
  }

  void create_swap_chain() {
    auto dxgi = d3d.as<IDXGIDevice>();
    com_ptr<IDXGIAdapter> adapter;
    check_hresult(dxgi->GetAdapter(adapter.put()));
    com_ptr<IDXGIFactory2> factory;
    check_hresult(adapter->GetParent(IID_PPV_ARGS(factory.put())));
    DXGI_SWAP_CHAIN_DESC1 description{};
    description.Width = pixel_width;
    description.Height = pixel_height;
    description.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    description.SampleDesc.Count = 1;
    description.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    description.BufferCount = 2;
    description.Scaling = DXGI_SCALING_STRETCH;
    description.SwapEffect = DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL;
    description.AlphaMode = DXGI_ALPHA_MODE_PREMULTIPLIED;
    check_hresult(factory->CreateSwapChainForComposition(d3d.get(), &description, nullptr, swap_chain.put()));
    apply_scale();
    check_hresult(panel.as<ISwapChainPanelNative>()->SetSwapChain(swap_chain.get()));
    create_target();
  }

  /* Returns false when the device was lost and the frame must wait. */
  bool prepare() {
    pixel_width = pixels(width, scale_x);
    pixel_height = pixels(height, scale_y);
    if (!device) create_device();
    if (!swap_chain) {
      create_swap_chain();
      return true;
    }
    D2D1_SIZE_U size = target ? target->GetPixelSize() : D2D1_SIZE_U{};
    if (size.width == pixel_width && size.height == pixel_height && target_scale_x == scale_x && target_scale_y == scale_y) return true;
    context->SetTarget(nullptr);
    target = nullptr;
    HRESULT result = swap_chain->ResizeBuffers(2, pixel_width, pixel_height, DXGI_FORMAT_B8G8R8A8_UNORM, 0);
    if (result == DXGI_ERROR_DEVICE_REMOVED || result == DXGI_ERROR_DEVICE_RESET) {
      lose_device();
      return false;
    }
    check_hresult(result);
    apply_scale();
    create_target();
    return true;
  }

  void draw() {
    if (drawing || !has_callback() || !callbacks.draw || width <= 0 || height <= 0) return;
    drawing = true;
    struct Reset {
      bool& value;
      ~Reset() { value = false; }
    } reset{drawing};
    double now = seconds();
    zpd_winui3_canvas_frame info{width, height, scale_x, now, last_time > 0 ? now - last_time : 0, title_bar_inset()};
    last_time = now;
    if (mode == ZPD_WINUI3_CANVAS_EXTERNAL) {
      callbacks.draw(callback.data, nullptr, &info);
      return;
    }
    try {
      if (!prepare()) {
        request_draw();
        return;
      }
    } catch (...) {
      lose_device();
      return;
    }
    context->BeginDraw();
    context->SetTransform(D2D1::Matrix3x2F::Identity());
    context->Clear(D2D1::ColorF(0, 0, 0, 0));
    zpd_winui3_draw draw{this, context.get(), {}, {}};
    callbacks.draw(callback.data, &draw, &info);
    draw.finish();
    HRESULT result = context->EndDraw();
    if (SUCCEEDED(result)) result = swap_chain->Present(1, 0);
    if (result == D2DERR_RECREATE_TARGET || result == DXGI_ERROR_DEVICE_REMOVED || result == DXGI_ERROR_DEVICE_RESET) {
      lose_device();
      request_draw();
      return;
    }
    // Images the frame did not draw are released.
    for (auto entry = images.begin(); entry != images.end();) {
      entry = entry->second.second == frame ? std::next(entry) : images.erase(entry);
    }
    ++frame;
  }

  void resized(double new_width, double new_height) {
    width = new_width;
    height = new_height;
    request_draw();
  }

  void rescaled() {
    scale_x = panel.CompositionScaleX();
    scale_y = panel.CompositionScaleY();
    if (proxy) {
      proxy->scale_x = scale_x;
      proxy->scale_y = scale_y;
      proxy->apply_scale();
    }
    request_draw();
  }

  /* Input. */

  static uint32_t modifiers() {
    uint32_t result{};
    if (GetKeyState(VK_SHIFT) < 0) result |= ZPD_WINUI3_MODIFIER_SHIFT;
    if (GetKeyState(VK_CONTROL) < 0) result |= ZPD_WINUI3_MODIFIER_CONTROL;
    if (GetKeyState(VK_MENU) < 0) result |= ZPD_WINUI3_MODIFIER_ALT;
    if (GetKeyState(VK_LWIN) < 0 || GetKeyState(VK_RWIN) < 0) result |= ZPD_WINUI3_MODIFIER_META;
    return result;
  }

  bool emit(zpd_winui3_canvas_input& input) {
    if (!has_callback() || !callbacks.input) return false;
    input.modifiers = modifiers();
    return callbacks.input(callback.data, &input);
  }

  static int32_t pressed(uinput::PointerUpdateKind kind) {
    switch (kind) {
      case uinput::PointerUpdateKind::LeftButtonPressed:
      case uinput::PointerUpdateKind::LeftButtonReleased: return 0;
      case uinput::PointerUpdateKind::RightButtonPressed:
      case uinput::PointerUpdateKind::RightButtonReleased: return 1;
      case uinput::PointerUpdateKind::MiddleButtonPressed:
      case uinput::PointerUpdateKind::MiddleButtonReleased: return 2;
      case uinput::PointerUpdateKind::XButton1Pressed:
      case uinput::PointerUpdateKind::XButton1Released: return 3;
      case uinput::PointerUpdateKind::XButton2Pressed:
      case uinput::PointerUpdateKind::XButton2Released: return 4;
      default: return -1;
    }
  }

  /* Counts presses of one button that follow within the double-click time and distance. */
  uint32_t count_click(int32_t button, Windows::Foundation::Point position) {
    double now = seconds();
    double limit = GetDoubleClickTime() / 1000.0;
    double dx = std::abs(position.X - last_click_position.X);
    double dy = std::abs(position.Y - last_click_position.Y);
    bool repeat = button == last_click_button && now - last_click_time <= limit && dx <= GetSystemMetrics(SM_CXDOUBLECLK) / 2.0 &&
                  dy <= GetSystemMetrics(SM_CYDOUBLECLK) / 2.0;
    click_count = repeat ? click_count + 1 : 1;
    last_click_button = button;
    last_click_time = now;
    last_click_position = position;
    return click_count;
  }

  void pointer_pressed(xinput::PointerRoutedEventArgs const& args) {
    auto current = args.GetCurrentPoint(host);
    auto position = current.Position();
    int32_t button = pressed(current.Properties().PointerUpdateKind());
    uint32_t count = count_click(button, position);
    // Presses under the title bar move the window unless the canvas claims them.
    if (button == 0 && position.Y < title_bar_inset() && !(callbacks.title_bar_hit && has_callback() && callbacks.title_bar_hit(callback.data, position.X, position.Y))) {
      if (HWND window = window_handle(host)) {
        POINT cursor{};
        GetCursorPos(&cursor);
        ReleaseCapture();
        PostMessageW(window, count == 2 ? WM_NCLBUTTONDBLCLK : WM_NCLBUTTONDOWN, HTCAPTION, MAKELPARAM(cursor.x, cursor.y));
      }
      args.Handled(true);
      return;
    }
    if (host.IsTabStop()) host.Focus(FocusState::Pointer);
    host.CapturePointer(args.Pointer());
    pressed_button = button;
    zpd_winui3_canvas_input input{};
    input.kind = ZPD_WINUI3_INPUT_POINTER_DOWN;
    input.x = position.X;
    input.y = position.Y;
    input.button = button;
    input.click_count = count;
    args.Handled(emit(input));
  }

  void pointer_released(xinput::PointerRoutedEventArgs const& args) {
    auto current = args.GetCurrentPoint(host);
    auto position = current.Position();
    int32_t button = pressed(current.Properties().PointerUpdateKind());
    zpd_winui3_canvas_input input{};
    input.kind = ZPD_WINUI3_INPUT_POINTER_UP;
    input.x = position.X;
    input.y = position.Y;
    input.button = button;
    input.click_count = click_count;
    pressed_button = -1;
    args.Handled(emit(input));
    host.ReleasePointerCapture(args.Pointer());
  }

  void pointer(xinput::PointerRoutedEventArgs const& args, int32_t kind) {
    auto position = args.GetCurrentPoint(host).Position();
    zpd_winui3_canvas_input input{};
    input.kind = kind;
    input.x = position.X;
    input.y = position.Y;
    input.button = kind == ZPD_WINUI3_INPUT_POINTER_MOVED ? pressed_button : -1;
    args.Handled(emit(input));
  }

  void wheel(xinput::PointerRoutedEventArgs const& args) {
    auto current = args.GetCurrentPoint(host);
    auto properties = current.Properties();
    double distance = properties.MouseWheelDelta() / 120.0 * WHEEL_NOTCH;
    zpd_winui3_canvas_input input{};
    input.kind = ZPD_WINUI3_INPUT_SCROLL;
    input.x = current.Position().X;
    input.y = current.Position().Y;
    input.button = -1;
    // A forward turn moves content down; a right tilt moves it left.
    if (properties.IsHorizontalMouseWheel()) input.delta_x = -distance;
    else input.delta_y = distance;
    input.precise = current.PointerDeviceType() == winrt::Microsoft::UI::Input::PointerDeviceType::Touchpad;
    args.Handled(emit(input));
  }

  static bool is_modifier(uint32_t key) {
    switch (key) {
      case VK_SHIFT: case VK_LSHIFT: case VK_RSHIFT:
      case VK_CONTROL: case VK_LCONTROL: case VK_RCONTROL:
      case VK_MENU: case VK_LMENU: case VK_RMENU:
      case VK_LWIN: case VK_RWIN: return true;
      default: return false;
    }
  }

  /* The text a key inserts with the current modifiers, without consuming dead keys. */
  static std::wstring key_text(uint32_t key, uint32_t scan) {
    BYTE state[256]{};
    if (!GetKeyboardState(state)) return {};
    bool control = state[VK_CONTROL] & 0x80;
    bool alt = state[VK_MENU] & 0x80;
    // Control shortcuts insert nothing, but AltGr (Control with Alt) types characters.
    if (control && !alt) return {};
    wchar_t buffer[8]{};
    int length = ToUnicodeEx(key, scan, state, buffer, 8, 0x4, GetKeyboardLayout(0));
    if (length <= 0) return {};
    std::wstring text(buffer, static_cast<size_t>(length));
    bool printable = std::all_of(text.begin(), text.end(), [](wchar_t ch) { return ch >= 0x20 && ch != 0x7f; });
    return printable ? text : std::wstring();
  }

  void key(xinput::KeyRoutedEventArgs const& args, bool down) {
    auto key = static_cast<uint32_t>(args.Key());
    auto status = args.KeyStatus();
    zpd_winui3_canvas_input input{};
    if (is_modifier(key)) {
      input.kind = ZPD_WINUI3_INPUT_MODIFIERS;
      emit(input);
      return;
    }
    input.kind = down ? ZPD_WINUI3_INPUT_KEY_DOWN : ZPD_WINUI3_INPUT_KEY_UP;
    input.key = key;
    input.code = status.ScanCode;
    input.repeat = down && status.WasKeyDown;
    input.button = -1;
    UINT character = MapVirtualKeyW(key, MAPVK_VK_TO_CHAR) & 0x7fffffff;
    input.character = character < 0x20 ? 0 : static_cast<uint32_t>(std::towlower(static_cast<wint_t>(character)));
    std::wstring text = down ? key_text(key, status.ScanCode) : std::wstring();
    // The character messages that follow this press are already reported here.
    if (down) pending_text = text;
    auto utf8 = to_string(text);
    input.text = {reinterpret_cast<uint8_t const*>(utf8.data()), utf8.size()};
    args.Handled(emit(input));
  }

  void character(xinput::CharacterReceivedRoutedEventArgs const& args) {
    wchar_t unit = static_cast<wchar_t>(args.Character());
    if (IS_HIGH_SURROGATE(unit)) {
      high_surrogate = unit;
      return;
    }
    std::wstring text;
    if (IS_LOW_SURROGATE(unit) && high_surrogate) text.push_back(high_surrogate);
    high_surrogate = 0;
    text.push_back(unit);
    if (pending_text.starts_with(text)) {
      pending_text.erase(0, text.size());
      return;
    }
    if (text.front() < 0x20 || text.front() == 0x7f) return;
    auto utf8 = to_string(text);
    zpd_winui3_canvas_input input{};
    input.kind = ZPD_WINUI3_INPUT_TEXT;
    input.button = -1;
    input.text = {reinterpret_cast<uint8_t const*>(utf8.data()), utf8.size()};
    args.Handled(emit(input));
  }

  /* Reports whether the host holds focus when that changes; focus events also bubble and repeat. */
  void focus_changed() {
    auto root = host.XamlRoot();
    bool focused = root && xinput::FocusManager::GetFocusedElement(root) == host;
    if (focused == reported_focus) return;
    reported_focus = focused;
    zpd_winui3_canvas_input input{};
    input.kind = ZPD_WINUI3_INPUT_FOCUS;
    input.button = -1;
    input.flag = focused;
    emit(input);
  }

  void set_focus(bool focused) {
    if (focused) {
      host.Focus(FocusState::Programmatic);
    } else if (host.FocusState() != FocusState::Unfocused) {
      // A canvas that stops taking focus loses it to the next element.
      bool tab_stop = host.IsTabStop();
      host.IsTabStop(false);
      host.IsTabStop(tab_stop);
    }
  }

  void set_cursor(int32_t cursor) {
    using Shape = uinput::InputSystemCursorShape;
    static constexpr Shape shapes[] = {Shape::Arrow, Shape::Hand, Shape::IBeam, Shape::Cross, Shape::SizeAll,
                                       Shape::SizeAll, Shape::SizeWestEast, Shape::SizeNorthSouth, Shape::UniversalNo};
    auto shape = shapes[std::clamp(cursor, 0, static_cast<int32_t>(std::size(shapes)) - 1)];
    host.as<IUIElementProtected>().ProtectedCursor(uinput::InputSystemCursor::Create(shape));
  }

  void install() {
    std::weak_ptr<CanvasState> weak = shared_from_this();
    auto with = [weak](auto body) {
      return [weak, body](IInspectable const&, auto const& args) {
        if (auto state = weak.lock()) body(*state, args);
      };
    };
    panel.SizeChanged(with([](CanvasState& state, SizeChangedEventArgs const& args) { state.resized(args.NewSize().Width, args.NewSize().Height); }));
    panel.CompositionScaleChanged([weak](SwapChainPanel const&, IInspectable const&) {
      if (auto state = weak.lock()) state->rescaled();
    });
    panel.Loaded(with([](CanvasState& state, RoutedEventArgs const&) {
      state.loaded = true;
      state.rescaled();
      state.update_rendering();
    }));
    panel.Unloaded(with([](CanvasState& state, RoutedEventArgs const&) {
      state.loaded = false;
      state.update_rendering();
    }));
    host.PointerPressed(with([](CanvasState& state, xinput::PointerRoutedEventArgs const& args) { state.pointer_pressed(args); }));
    host.PointerReleased(with([](CanvasState& state, xinput::PointerRoutedEventArgs const& args) { state.pointer_released(args); }));
    host.PointerMoved(with([](CanvasState& state, xinput::PointerRoutedEventArgs const& args) { state.pointer(args, ZPD_WINUI3_INPUT_POINTER_MOVED); }));
    host.PointerEntered(with([](CanvasState& state, xinput::PointerRoutedEventArgs const& args) { state.pointer(args, ZPD_WINUI3_INPUT_POINTER_ENTERED); }));
    host.PointerExited(with([](CanvasState& state, xinput::PointerRoutedEventArgs const& args) { state.pointer(args, ZPD_WINUI3_INPUT_POINTER_EXITED); }));
    host.PointerCaptureLost(with([](CanvasState& state, xinput::PointerRoutedEventArgs const&) { state.pressed_button = -1; }));
    host.PointerWheelChanged(with([](CanvasState& state, xinput::PointerRoutedEventArgs const& args) { state.wheel(args); }));
    host.KeyDown(with([](CanvasState& state, xinput::KeyRoutedEventArgs const& args) { state.key(args, true); }));
    host.KeyUp(with([](CanvasState& state, xinput::KeyRoutedEventArgs const& args) { state.key(args, false); }));
    host.CharacterReceived(with([](CanvasState& state, xinput::CharacterReceivedRoutedEventArgs const& args) { state.character(args); }));
    host.GotFocus(with([](CanvasState& state, RoutedEventArgs const&) { state.focus_changed(); }));
    host.LostFocus(with([](CanvasState& state, RoutedEventArgs const&) { state.focus_changed(); }));
  }
};

struct CanvasElement final : zpd_winui3_element {
  std::shared_ptr<CanvasState> state;

  CanvasElement(Grid const& host, std::shared_ptr<CanvasState> state) : zpd_winui3_element(host), state(std::move(state)) {}

  ~CanvasElement() override {
    try {
      state->stop_rendering();
      state->callback.reset();
      state->lose_device();
    } catch (...) {
    }
  }
};

CanvasElement* canvas(zpd_winui3_element* element) { return static_cast<CanvasElement*>(element); }

}  // namespace

struct zpd_winui3_canvas_handle {
  std::weak_ptr<CanvasState> state;
};

void zpd_winui3_draw::restore() {
  if (stack.empty()) return;
  for (; state.layers > 0; --state.layers) context->PopLayer();
  state = stack.back();
  stack.pop_back();
  context->SetTransform(state.transform);
  context->SetAntialiasMode(state.antialias ? D2D1_ANTIALIAS_MODE_PER_PRIMITIVE : D2D1_ANTIALIAS_MODE_ALIASED);
}

void zpd_winui3_draw::finish() {
  while (!stack.empty()) restore();
  for (; state.layers > 0; --state.layers) context->PopLayer();
}

com_ptr<ID2D1Brush> zpd_winui3_draw::brush(zpd_winui3_paint const& paint) {
  if (paint.kind == ZPD_WINUI3_PAINT_SOLID) {
    canvas->solid->SetColor(color_f(paint.color));
    canvas->solid->SetOpacity(state.alpha);
    return canvas->solid.as<ID2D1Brush>();
  }
  if (!paint.stops_length) return nullptr;
  std::vector<D2D1_GRADIENT_STOP> stops;
  for (size_t index = 0; index < paint.stops_length; ++index) {
    stops.push_back({static_cast<float>(paint.stops[index].offset), color_f(paint.stops[index].color)});
  }
  com_ptr<ID2D1GradientStopCollection> collection;
  check_hresult(context->CreateGradientStopCollection(stops.data(), static_cast<UINT32>(stops.size()), D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP, collection.put()));
  auto properties = D2D1::BrushProperties(state.alpha);
  if (paint.kind == ZPD_WINUI3_PAINT_LINEAR) {
    com_ptr<ID2D1LinearGradientBrush> result;
    check_hresult(context->CreateLinearGradientBrush(D2D1::LinearGradientBrushProperties(point(paint.x0, paint.y0), point(paint.x1, paint.y1)), properties, collection.get(), result.put()));
    return result.as<ID2D1Brush>();
  }
  float radius = static_cast<float>(paint.radius);
  com_ptr<ID2D1RadialGradientBrush> result;
  check_hresult(context->CreateRadialGradientBrush(D2D1::RadialGradientBrushProperties(point(paint.x0, paint.y0), {0, 0}, radius, radius), properties, collection.get(), result.put()));
  return result.as<ID2D1Brush>();
}

/* Runs a drawing call; native failures skip the call instead of crossing the C boundary. */
template <typename F>
void guarded(F&& body) noexcept {
  try {
    body();
  } catch (...) {
  }
}

extern "C" {

zpd_winui3_element* zpd_winui3_canvas_create(int32_t mode) {
  return create<zpd_winui3_element>([&] {
    SwapChainPanel panel;
    Grid host;
    host.Background(Media::SolidColorBrush(Windows::UI::Colors::Transparent()));
    host.Children().Append(panel);
    auto state = std::make_shared<CanvasState>();
    state->mode = mode;
    state->panel = panel;
    state->host = host;
    state->scale_x = panel.CompositionScaleX();
    state->scale_y = panel.CompositionScaleY();
    host.IsTabStop(false);
    if (mode == ZPD_WINUI3_CANVAS_EXTERNAL) {
      state->proxy = make_self<SwapChainPanelProxy>();
      state->proxy->panel = panel.as<ISwapChainPanelNative>();
      state->proxy->scale_x = state->scale_x;
      state->proxy->scale_y = state->scale_y;
      state->proxy_native = state->proxy.as<ISwapChainPanelNative>();
    }
    state->install();
    return new CanvasElement(host, state);
  });
}

int32_t zpd_winui3_canvas_set_callbacks(zpd_winui3_element* target, const void* data, zpd_winui3_canvas_callbacks callbacks, zpd_winui3_release_fn release) {
  Callback callback(data, release);
  return status([&] {
    auto state = canvas(target)->state;
    state->callbacks = {};
    state->callback = std::move(callback);
    state->callbacks = callbacks;
    state->request_draw();
  });
}

int32_t zpd_winui3_canvas_set_redraw(zpd_winui3_element* target, bool continuous, double fps) {
  return status([&] {
    auto state = canvas(target)->state;
    state->continuous = continuous;
    state->fps = fps;
    state->update_rendering();
    state->request_draw();
  });
}

int32_t zpd_winui3_canvas_set_focusable(zpd_winui3_element* target, bool focusable) {
  return status([&] { canvas(target)->state->host.IsTabStop(focusable); });
}

void* zpd_winui3_canvas_swap_chain_panel(zpd_winui3_element* target) {
  return canvas(target)->state->proxy_native.get();
}

zpd_winui3_canvas_handle* zpd_winui3_canvas_handle_create(const zpd_winui3_element* target) {
  return create<zpd_winui3_canvas_handle>([&] { return new zpd_winui3_canvas_handle{static_cast<CanvasElement const*>(target)->state}; });
}

void zpd_winui3_canvas_handle_release(zpd_winui3_canvas_handle* handle) { delete handle; }

void zpd_winui3_canvas_handle_invalidate(const zpd_winui3_canvas_handle* handle) {
  guarded([&] {
    if (auto state = handle->state.lock()) state->request_draw();
  });
}

void zpd_winui3_canvas_handle_set_focus(const zpd_winui3_canvas_handle* handle, bool focused) {
  guarded([&] {
    if (auto state = handle->state.lock()) state->set_focus(focused);
  });
}

void zpd_winui3_canvas_handle_set_cursor(const zpd_winui3_canvas_handle* handle, int32_t cursor) {
  guarded([&] {
    if (auto state = handle->state.lock()) state->set_cursor(cursor);
  });
}

void zpd_winui3_draw_save(zpd_winui3_draw* draw) {
  guarded([&] {
    draw->stack.push_back(draw->state);
    draw->state.layers = 0;
  });
}

void zpd_winui3_draw_restore(zpd_winui3_draw* draw) {
  guarded([&] { draw->restore(); });
}

void zpd_winui3_draw_concat(zpd_winui3_draw* draw, zpd_winui3_transform t) {
  guarded([&] {
    D2D1_MATRIX_3X2_F matrix{static_cast<float>(t.a), static_cast<float>(t.b), static_cast<float>(t.c),
                             static_cast<float>(t.d), static_cast<float>(t.tx), static_cast<float>(t.ty)};
    // Row vectors: the new transform applies first.
    draw->state.transform = D2D1::Matrix3x2F::ReinterpretBaseType(&matrix)[0] * D2D1::Matrix3x2F::ReinterpretBaseType(&draw->state.transform)[0];
    draw->context->SetTransform(draw->state.transform);
  });
}

void zpd_winui3_draw_set_alpha(zpd_winui3_draw* draw, double alpha) {
  draw->state.alpha = static_cast<float>(std::clamp(alpha, 0.0, 1.0));
}

void zpd_winui3_draw_set_antialias(zpd_winui3_draw* draw, bool enabled) {
  draw->state.antialias = enabled;
  draw->context->SetAntialiasMode(enabled ? D2D1_ANTIALIAS_MODE_PER_PRIMITIVE : D2D1_ANTIALIAS_MODE_ALIASED);
}

void zpd_winui3_draw_clip(zpd_winui3_draw* draw, const zpd_winui3_path* path) {
  guarded([&] {
    auto mask = geometry(*path);
    auto parameters = D2D1::LayerParameters1(D2D1::InfiniteRect(), mask.get(),
                                             draw->state.antialias ? D2D1_ANTIALIAS_MODE_PER_PRIMITIVE : D2D1_ANTIALIAS_MODE_ALIASED);
    draw->context->PushLayer(parameters, nullptr);
    ++draw->state.layers;
  });
}

void zpd_winui3_draw_clear(zpd_winui3_draw* draw, double x, double y, double width, double height, zpd_winui3_color color) {
  guarded([&] {
    auto brush = draw->canvas->solid;
    brush->SetColor(color_f(color));
    brush->SetOpacity(1);
    // Copying replaces the pixels, including alpha, instead of blending.
    draw->context->SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_COPY);
    draw->context->FillRectangle(D2D1::RectF(static_cast<float>(x), static_cast<float>(y), static_cast<float>(x + width), static_cast<float>(y + height)), brush.get());
    draw->context->SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_SOURCE_OVER);
  });
}

void zpd_winui3_draw_fill(zpd_winui3_draw* draw, const zpd_winui3_path* path, const zpd_winui3_paint* paint) {
  guarded([&] {
    auto brush = draw->brush(*paint);
    if (!brush) return;
    draw->context->FillGeometry(geometry(*path).get(), brush.get());
  });
}

void zpd_winui3_draw_stroke(zpd_winui3_draw* draw, const zpd_winui3_path* path, const zpd_winui3_paint* paint, const zpd_winui3_stroke* stroke) {
  guarded([&] {
    auto brush = draw->brush(*paint);
    if (!brush) return;
    draw->context->DrawGeometry(geometry(*path).get(), brush.get(), static_cast<float>(stroke->width), stroke_style(*stroke).get());
  });
}

void zpd_winui3_draw_text(zpd_winui3_draw* draw, zpd_winui3_string text, double x, double y, const zpd_winui3_font* font, zpd_winui3_color color) {
  guarded([&] {
    if (!text.length) return;
    auto layout = fonts().layout(text, FontKey::from(*font));
    DWRITE_LINE_METRICS line{};
    UINT32 count{};
    layout->GetLineMetrics(&line, 1, &count);
    auto brush = draw->canvas->solid;
    brush->SetColor(color_f(color));
    brush->SetOpacity(draw->state.alpha);
    draw->context->DrawTextLayout(point(x, y - line.baseline), layout.get(), brush.get(), D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT);
  });
}

void zpd_winui3_draw_image(zpd_winui3_draw* draw, uint64_t id, uint32_t width, uint32_t height, const uint8_t* pixels, double x, double y, double rect_width, double rect_height) {
  guarded([&] {
    auto* canvas = draw->canvas;
    auto& slot = canvas->images[id];
    if (!slot.first) {
      // Direct2D takes premultiplied BGRA.
      std::vector<uint8_t> converted(static_cast<size_t>(width) * height * 4);
      for (size_t index = 0; index < converted.size(); index += 4) {
        uint32_t alpha = pixels[index + 3];
        converted[index + 0] = static_cast<uint8_t>((pixels[index + 2] * alpha + 127) / 255);
        converted[index + 1] = static_cast<uint8_t>((pixels[index + 1] * alpha + 127) / 255);
        converted[index + 2] = static_cast<uint8_t>((pixels[index + 0] * alpha + 127) / 255);
        converted[index + 3] = static_cast<uint8_t>(alpha);
      }
      auto properties = D2D1::BitmapProperties1(D2D1_BITMAP_OPTIONS_NONE, D2D1::PixelFormat(DXGI_FORMAT_B8G8R8A8_UNORM, D2D1_ALPHA_MODE_PREMULTIPLIED));
      check_hresult(draw->context->CreateBitmap({width, height}, converted.data(), width * 4, properties, slot.first.put()));
    }
    slot.second = canvas->frame;
    auto rect = D2D1::RectF(static_cast<float>(x), static_cast<float>(y), static_cast<float>(x + rect_width), static_cast<float>(y + rect_height));
    draw->context->DrawBitmap(slot.first.get(), &rect, draw->state.alpha, D2D1_INTERPOLATION_MODE_LINEAR, nullptr, nullptr);
  });
}

int32_t zpd_winui3_measure_text(zpd_winui3_string text, const zpd_winui3_font* font, zpd_winui3_text_metrics* metrics) {
  return status([&] {
    auto key = FontKey::from(*font);
    auto const& font_metrics = fonts().font_metrics(key);
    double width = 0;
    if (text.length) {
      DWRITE_TEXT_METRICS measured{};
      check_hresult(fonts().layout(text, key)->GetMetrics(&measured));
      width = measured.widthIncludingTrailingWhitespace;
    }
    *metrics = {width, font_metrics.ascent, font_metrics.descent, font_metrics.leading};
  });
}

}  // extern "C"
