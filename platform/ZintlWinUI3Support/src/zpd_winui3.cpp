#include "common.h"

#include <MddBootstrap.h>
#include <microsoft.ui.xaml.window.h>

#include <algorithm>
#include <cctype>
#include <mutex>
#include <vector>

#include <winrt/Microsoft.UI.Xaml.Hosting.h>
#include <winrt/Microsoft.UI.Xaml.Markup.h>
#include <winrt/Microsoft.UI.Xaml.XamlTypeInfo.h>
#include <winrt/Windows.System.h>

using namespace zpd;
using namespace winrt::Microsoft::UI;
using namespace winrt::Microsoft::UI::Dispatching;
using namespace winrt::Microsoft::UI::Xaml::Markup;

namespace zpd {

thread_local std::string last_error;

Media::SolidColorBrush brush_from(zpd_winui3_color color) {
  auto channel = [](double value) {
    return static_cast<uint8_t>(std::clamp(value, 0.0, 1.0) * 255.0 + 0.5);
  };
  return Media::SolidColorBrush(Windows::UI::Color{
      channel(color.alpha), channel(color.red), channel(color.green), channel(color.blue)});
}

Windows::System::VirtualKey key_from(zpd_winui3_string key) {
  auto value = bytes(key);
  if (value.size() == 1) {
    char character = static_cast<char>(std::toupper(static_cast<unsigned char>(value[0])));
    if (character >= 'A' && character <= 'Z') return static_cast<Windows::System::VirtualKey>(character);
    if (character >= '0' && character <= '9') return static_cast<Windows::System::VirtualKey>(character);
  }
  if (value == "enter") return Windows::System::VirtualKey::Enter;
  if (value == "escape") return Windows::System::VirtualKey::Escape;
  if (value == "delete") return Windows::System::VirtualKey::Delete;
  if (value == "space") return Windows::System::VirtualKey::Space;
  throw hresult_invalid_argument(L"unsupported keyboard accelerator key");
}

Windows::System::VirtualKeyModifiers modifiers_from(uint32_t value) {
  auto result = Windows::System::VirtualKeyModifiers::None;
  if (value & 1) result = result | Windows::System::VirtualKeyModifiers::Control;
  if (value & 2) result = result | Windows::System::VirtualKeyModifiers::Menu;
  if (value & 4) result = result | Windows::System::VirtualKeyModifiers::Shift;
  if (value & 8) result = result | Windows::System::VirtualKeyModifiers::Windows;
  return result;
}

void configure_accelerator(MenuFlyoutItem const& item, zpd_winui3_menu_item const& source) {
  if (!source.key.length) return;
  Xaml::Input::KeyboardAccelerator accelerator;
  accelerator.Key(key_from(source.key));
  accelerator.Modifiers(modifiers_from(source.modifiers));
  item.KeyboardAccelerators().Append(accelerator);
}

/* Applies a standard editing action to the focused text box. */
void perform_role(int32_t role, XamlRoot const& root) {
  if (!root) return;
  auto box = Xaml::Input::FocusManager::GetFocusedElement(root).try_as<TextBox>();
  if (!box) return;
  switch (role) {
    case ZPD_WINUI3_MENU_ROLE_CUT: box.CutSelectionToClipboard(); break;
    case ZPD_WINUI3_MENU_ROLE_COPY: box.CopySelectionToClipboard(); break;
    case ZPD_WINUI3_MENU_ROLE_PASTE: box.PasteFromClipboard(); break;
    case ZPD_WINUI3_MENU_ROLE_SELECT_ALL: box.SelectAll(); break;
    case ZPD_WINUI3_MENU_ROLE_UNDO: box.Undo(); break;
    case ZPD_WINUI3_MENU_ROLE_REDO: box.Redo(); break;
    case ZPD_WINUI3_MENU_ROLE_DELETE: box.SelectedText(L""); break;
    default: break;
  }
}

void append_menu_items(
    Windows::Foundation::Collections::IVector<MenuFlyoutItemBase> const& target,
    std::shared_ptr<MenuTarget> const& menu_target,
    const zpd_winui3_menu_item* items,
    size_t length) {
  for (size_t index = 0; index < length; ++index) {
    auto const& source = items[index];
    if (source.kind == ZPD_WINUI3_MENU_SEPARATOR) {
      target.Append(MenuFlyoutSeparator());
      continue;
    }
    if (source.kind == ZPD_WINUI3_MENU_SUBMENU) {
      MenuFlyoutSubItem submenu;
      submenu.Text(string_from(source.title));
      submenu.IsEnabled(source.enabled);
      append_menu_items(submenu.Items(), menu_target, source.children, source.children_length);
      target.Append(submenu);
      continue;
    }
    MenuFlyoutItem item;
    item.Text(string_from(source.title));
    item.IsEnabled(source.enabled);
    configure_accelerator(item, source);
    auto id = std::string(bytes(source.id));
    auto role = source.role;
    std::weak_ptr<MenuTarget> weak = menu_target;
    item.Click([weak, id = std::move(id), role](Windows::Foundation::IInspectable const& sender, RoutedEventArgs const&) {
      if (role != ZPD_WINUI3_MENU_ROLE_NONE) {
        perform_role(role, sender.as<UIElement>().XamlRoot());
        return;
      }
      if (auto target = weak.lock()) target->select(id);
    });
    target.Append(item);
  }
}

}  // namespace zpd

struct zpd_winui3_dispatcher {
  DispatcherQueue queue{nullptr};
};

struct DispatcherSourceState {
  std::mutex mutex;
  DispatcherQueue queue{nullptr};
  const void* data{};
  zpd_winui3_invoke_fn invoke{};
  zpd_winui3_release_fn release{};
  bool active{true};
  bool scheduled{};
};

struct zpd_winui3_dispatcher_source {
  std::shared_ptr<DispatcherSourceState> state;
};

struct zpd_winui3_dispatcher_signaler {
  std::weak_ptr<DispatcherSourceState> state;
};

struct zpd_winui3_app_context {
  DispatcherQueue queue{nullptr};
};

struct ButtonElement final : zpd_winui3_element {
  Button button{nullptr};
  event_token click_token{};
  Callback callback;
  explicit ButtonElement(Button const& value) : zpd_winui3_element(value), button(value) {}
  ~ButtonElement() override {
    if (click_token.value) button.Click(click_token);
  }
  void clear() {
    if (click_token.value) {
      button.Click(click_token);
      click_token = {};
    }
    callback.reset();
  }
};

struct TextBoxElement final : zpd_winui3_element {
  TextBox text_box{nullptr};
  event_token change_token{};
  Callback callback;
  explicit TextBoxElement(TextBox const& value) : zpd_winui3_element(value), text_box(value) {}
  ~TextBoxElement() override {
    if (change_token.value) text_box.TextChanged(change_token);
  }
  void clear() {
    if (change_token.value) {
      text_box.TextChanged(change_token);
      change_token = {};
    }
    callback.reset();
  }
};

zpd_winui3_window::zpd_winui3_window(int32_t style) {
  window = Window();
  shell = Grid();
  menu_bar = MenuBar();

  RowDefinition menu_row;
  menu_row.Height(GridLengthHelper::Auto());
  RowDefinition content_row;
  content_row.Height(GridLengthHelper::FromValueAndType(1.0, GridUnitType::Star));
  shell.RowDefinitions().Append(menu_row);
  shell.RowDefinitions().Append(content_row);
  shell.Children().Append(menu_bar);
  menu_bar.Visibility(Visibility::Collapsed);
  window.Content(shell);

  if (style == ZPD_WINUI3_WINDOW_PANEL) {
    auto presenter = Windowing::OverlappedPresenter::Create();
    presenter.IsAlwaysOnTop(true);
    presenter.IsMinimizable(false);
    presenter.IsMaximizable(false);
    window.AppWindow().SetPresenter(presenter);
    window.AppWindow().IsShownInSwitchers(false);
  }
  closed_token = window.Closed([this](auto&&, auto&&) {
    closed = true;
    if (closed_invoke && closed_callback.data) closed_invoke(closed_callback.data);
  });
}

HWND zpd_winui3_window::hwnd() const {
  HWND handle{};
  check_hresult(window.as<IWindowNative>()->get_WindowHandle(&handle));
  return handle;
}

XamlRoot zpd_winui3_window::xaml_root() const { return shell.XamlRoot(); }

struct LaunchState {
  const void* data;
  zpd_winui3_launch_fn launch;
};

LaunchState* launch_state{};

struct NativeApplication : ApplicationT<NativeApplication, IXamlMetadataProvider> {
  std::vector<IXamlMetadataProvider> providers;

  NativeApplication() {
    Hosting::WindowsXamlManager::InitializeForCurrentThread();
    Resources().MergedDictionaries().Append(XamlControlsResources());
    // Compact density keeps control heights close to the declared desktop metrics.
    try {
      ResourceDictionary compact;
      compact.Source(Windows::Foundation::Uri(L"ms-appx:///Microsoft.UI.Xaml/DensityStyles/Compact.xaml"));
      Resources().MergedDictionaries().Append(compact);
    } catch (...) {
    }
  }

  void OnLaunched(LaunchActivatedEventArgs const&) {
    if (!launch_state || !launch_state->launch) return;
    zpd_winui3_app_context context{DispatcherQueue::GetForCurrentThread()};
    launch_state->launch(&context, launch_state->data);
  }

  std::vector<IXamlMetadataProvider> const& Providers() {
    if (providers.empty()) providers.push_back(XamlTypeInfo::XamlControlsXamlMetaDataProvider());
    return providers;
  }

  IXamlType GetXamlType(Windows::UI::Xaml::Interop::TypeName const& type) {
    for (auto const& provider : Providers()) if (auto result = provider.GetXamlType(type)) return result;
    return nullptr;
  }

  IXamlType GetXamlType(hstring const& name) {
    for (auto const& provider : Providers()) if (auto result = provider.GetXamlType(name)) return result;
    return nullptr;
  }

  com_array<XmlnsDefinition> GetXmlnsDefinitions() {
    std::vector<XmlnsDefinition> definitions;
    for (auto const& provider : Providers()) {
      auto values = provider.GetXmlnsDefinitions();
      definitions.insert(definitions.end(), values.begin(), values.end());
    }
    return {definitions.begin(), definitions.end()};
  }
};

GridLength grid_length(zpd_winui3_grid_length value) {
  switch (value.kind) {
    case 0: return GridLengthHelper::Auto();
    case 1: return GridLengthHelper::FromPixels(value.value);
    case 2: return GridLengthHelper::FromValueAndType(value.value, GridUnitType::Star);
    default: throw hresult_invalid_argument(L"invalid GridLength kind");
  }
}

extern "C" {

int32_t zpd_winui3_application_run(const void* data, zpd_winui3_launch_fn launch, zpd_winui3_release_fn release) {
  bool released{};
  int32_t result = status([&] {
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    PACKAGE_VERSION minimum{};
    check_hresult(MddBootstrapInitialize2(0x00010008, L"", minimum, MddBootstrapInitializeOptions_OnNoMatch_ShowUI));
    bool apartment_initialized{};
    try {
      init_apartment(apartment_type::single_threaded);
      apartment_initialized = true;
      LaunchState state{data, launch};
      launch_state = &state;
      Application::Start([](auto&&) { make<NativeApplication>(); });
      launch_state = nullptr;
      if (data && release) {
        release(data);
        released = true;
      }
      if (apartment_initialized) uninit_apartment();
    } catch (...) {
      launch_state = nullptr;
      if (data && release) {
        release(data);
        released = true;
      }
      if (apartment_initialized) uninit_apartment();
      MddBootstrapShutdown();
      throw;
    }
    MddBootstrapShutdown();
  });
  if (!released && data && release) release(data);
  return result;
}

zpd_winui3_dispatcher* zpd_winui3_app_dispatcher(const zpd_winui3_app_context* context) {
  if (!context) return nullptr;
  return create<zpd_winui3_dispatcher>([&] { return new zpd_winui3_dispatcher{context->queue}; });
}

zpd_winui3_string zpd_winui3_last_error() {
  return {reinterpret_cast<uint8_t const*>(last_error.data()), last_error.size()};
}

int32_t zpd_winui3_application_exit() {
  return status([] { Application::Current().Exit(); });
}

zpd_winui3_window* zpd_winui3_window_create_styled(int32_t style) {
  return create<zpd_winui3_window>([&] { return new zpd_winui3_window(style); });
}

zpd_winui3_dispatcher* zpd_winui3_dispatcher_clone(const zpd_winui3_dispatcher* dispatcher) {
  if (!dispatcher) return nullptr;
  return create<zpd_winui3_dispatcher>([&] { return new zpd_winui3_dispatcher{dispatcher->queue}; });
}

void zpd_winui3_dispatcher_release(zpd_winui3_dispatcher* dispatcher) { delete dispatcher; }

bool zpd_winui3_dispatcher_try_enqueue(const zpd_winui3_dispatcher* dispatcher, int32_t priority, const void* data, zpd_winui3_invoke_fn invoke, zpd_winui3_release_fn release) {
  if (!dispatcher || !invoke) { if (data && release) release(data); return false; }
  auto callback = std::make_shared<Callback>(data, release);
  try {
    auto native_priority = static_cast<DispatcherQueuePriority>(priority);
    return dispatcher->queue.TryEnqueue(native_priority, [callback, invoke] { invoke(callback->data); });
  } catch (...) {
    return false;
  }
}

zpd_winui3_dispatcher_source* zpd_winui3_dispatcher_source_create(const zpd_winui3_dispatcher* dispatcher, const void* data, zpd_winui3_invoke_fn invoke, zpd_winui3_release_fn release) {
  if (!dispatcher || !invoke) {
    if (data && release) release(data);
    return nullptr;
  }
  try {
    auto state = std::make_shared<DispatcherSourceState>();
    state->queue = dispatcher->queue;
    state->data = data;
    state->invoke = invoke;
    state->release = release;
    return new zpd_winui3_dispatcher_source{std::move(state)};
  } catch (...) {
    if (data && release) release(data);
    return nullptr;
  }
}

void zpd_winui3_dispatcher_source_release(zpd_winui3_dispatcher_source* source) {
  if (!source) return;
  const void* data{};
  zpd_winui3_release_fn release{};
  {
    std::scoped_lock lock(source->state->mutex);
    source->state->active = false;
    source->state->scheduled = false;
    data = std::exchange(source->state->data, nullptr);
    release = std::exchange(source->state->release, nullptr);
    source->state->invoke = nullptr;
  }
  if (data && release) release(data);
  delete source;
}

zpd_winui3_dispatcher_signaler* zpd_winui3_dispatcher_source_signaler(const zpd_winui3_dispatcher_source* source) {
  if (!source) return nullptr;
  return create<zpd_winui3_dispatcher_signaler>([&] { return new zpd_winui3_dispatcher_signaler{source->state}; });
}

zpd_winui3_dispatcher_signaler* zpd_winui3_dispatcher_signaler_clone(const zpd_winui3_dispatcher_signaler* signaler) {
  if (!signaler) return nullptr;
  return create<zpd_winui3_dispatcher_signaler>([&] { return new zpd_winui3_dispatcher_signaler{signaler->state}; });
}

void zpd_winui3_dispatcher_signaler_release(zpd_winui3_dispatcher_signaler* signaler) { delete signaler; }

bool zpd_winui3_dispatcher_signaler_signal(const zpd_winui3_dispatcher_signaler* signaler) {
  if (!signaler) return false;
  auto state = signaler->state.lock();
  if (!state) return false;
  {
    std::scoped_lock lock(state->mutex);
    if (!state->active) return false;
    if (state->scheduled) return true;
    state->scheduled = true;
  }
  try {
    auto weak = std::weak_ptr<DispatcherSourceState>(state);
    if (state->queue.TryEnqueue(DispatcherQueuePriority::Normal, [weak] {
      auto state = weak.lock();
      if (!state) return;
      const void* data{};
      zpd_winui3_invoke_fn invoke{};
      {
        std::scoped_lock lock(state->mutex);
        state->scheduled = false;
        if (!state->active) return;
        data = state->data;
        invoke = state->invoke;
      }
      if (data && invoke) invoke(data);
    })) {
      return true;
    }
  } catch (...) {
  }
  std::scoped_lock lock(state->mutex);
  state->scheduled = false;
  return false;
}

void zpd_winui3_window_release(zpd_winui3_window* window) {
  if (!window) return;
  try {
    window->closed_invoke = nullptr;
    window->closed_callback.reset();
    window->window.Closed(window->closed_token);
    window->menu_bar.Items().Clear();
    window->menu_target.reset();
    if (!window->closed) window->window.Close();
  } catch (...) {
  }
  delete window;
}
int32_t zpd_winui3_window_set_title(zpd_winui3_window* window, zpd_winui3_string title) { return status([&] { window->window.Title(string_from(title)); }); }
int32_t zpd_winui3_window_resize(zpd_winui3_window* window, int32_t width, int32_t height) { return status([&] { window->window.AppWindow().Resize({width, height}); }); }
int32_t zpd_winui3_window_activate(zpd_winui3_window* window) { return status([&] { window->window.Activate(); }); }
int32_t zpd_winui3_window_close(zpd_winui3_window* window) { return status([&] { if (!window->closed) window->window.Close(); }); }
int32_t zpd_winui3_window_extend_content_into_title_bar(zpd_winui3_window* window, bool enabled) { return status([&] { window->window.ExtendsContentIntoTitleBar(enabled); }); }
int32_t zpd_winui3_window_set_title_bar(zpd_winui3_window* window, const zpd_winui3_element* element) { return status([&] { window->window.SetTitleBar(element ? element->value : nullptr); }); }

int32_t zpd_winui3_window_move_and_resize(zpd_winui3_window* window, double x, double y, double width, double height) {
  return status([&] {
    double scale = GetDpiForWindow(window->hwnd()) / 96.0;
    auto pixels = [scale](double value) { return static_cast<int32_t>(value * scale + 0.5); };
    // Declared bounds describe the content area, as on macOS.
    window->window.AppWindow().Move({pixels(x), pixels(y)});
    window->window.AppWindow().ResizeClient({pixels(width), pixels(height)});
  });
}

int32_t zpd_winui3_window_set_closed_handler(zpd_winui3_window* window, const void* data, zpd_winui3_invoke_fn invoke, zpd_winui3_release_fn release) {
  return status([&] {
    window->closed_callback = Callback(data, release);
    window->closed_invoke = invoke;
  });
}

bool zpd_winui3_window_has_sheet(const zpd_winui3_window* window) { return window && *window->sheets > 0; }

int32_t zpd_winui3_window_set_content(zpd_winui3_window* window, const zpd_winui3_element* element) {
  return status([&] {
    if (window->content) {
      uint32_t index{};
      if (window->shell.Children().IndexOf(window->content, index)) window->shell.Children().RemoveAt(index);
    }
    window->content = element ? element->value : nullptr;
    if (window->content) {
      Grid::SetRow(window->content, 1);
      window->shell.Children().Append(window->content);
    }
  });
}

int32_t zpd_winui3_window_set_backdrop(zpd_winui3_window* window, int32_t backdrop) {
  return status([&] {
    if (backdrop == 0) window->window.SystemBackdrop(nullptr);
    else if (backdrop == 1) window->window.SystemBackdrop(Media::MicaBackdrop());
    else if (backdrop == 2) window->window.SystemBackdrop(Media::DesktopAcrylicBackdrop());
    else throw hresult_invalid_argument(L"invalid SystemBackdrop value");
  });
}

int32_t zpd_winui3_window_clear_menu_bar(zpd_winui3_window* window) {
  return status([&] {
    window->menu_bar.Items().Clear();
    window->menu_bar.Visibility(Visibility::Collapsed);
    window->menu_target.reset();
  });
}

int32_t zpd_winui3_window_set_menu_bar(zpd_winui3_window* window, const zpd_winui3_menu* menus, size_t length, const void* data, zpd_winui3_string_fn invoke, zpd_winui3_release_fn release) {
  return status([&] {
    window->menu_bar.Items().Clear();
    auto target = std::make_shared<MenuTarget>();
    target->callback = Callback(data, release);
    target->invoke = invoke;
    window->menu_target = target;
    for (size_t index = 0; index < length; ++index) {
      MenuBarItem menu;
      menu.Title(string_from(menus[index].title));
      append_menu_items(menu.Items(), target, menus[index].items, menus[index].items_length);
      window->menu_bar.Items().Append(menu);
    }
    window->menu_bar.Visibility(length ? Visibility::Visible : Visibility::Collapsed);
  });
}

zpd_winui3_element* zpd_winui3_button_create(zpd_winui3_string title) { return create<zpd_winui3_element>([&] { Button button; button.Content(box_value(string_from(title))); return new ButtonElement(button); }); }
int32_t zpd_winui3_button_set_title(zpd_winui3_element* element, zpd_winui3_string title) { return status([&] { static_cast<ButtonElement*>(element)->button.Content(box_value(string_from(title))); }); }
int32_t zpd_winui3_button_set_enabled(zpd_winui3_element* element, bool enabled) { return status([&] { static_cast<ButtonElement*>(element)->button.IsEnabled(enabled); }); }
int32_t zpd_winui3_button_clear_click_handler(zpd_winui3_element* element) { return status([&] { static_cast<ButtonElement*>(element)->clear(); }); }
int32_t zpd_winui3_button_set_click_handler(zpd_winui3_element* element, const void* data, zpd_winui3_invoke_fn invoke, zpd_winui3_release_fn release) {
  return status([&] { auto button = static_cast<ButtonElement*>(element); button->clear(); button->callback = Callback(data, release); button->click_token = button->button.Click([button, invoke](auto&&, auto&&) { invoke(button->callback.data); }); });
}

zpd_winui3_element* zpd_winui3_text_create(zpd_winui3_string text) { return create<zpd_winui3_element>([&] { TextBlock value; value.Text(string_from(text)); return new zpd_winui3_element(value); }); }
int32_t zpd_winui3_text_set_text(zpd_winui3_element* element, zpd_winui3_string value) { return status([&] { element->value.as<TextBlock>().Text(string_from(value)); }); }
int32_t zpd_winui3_text_set_wrapping(zpd_winui3_element* element, bool enabled) { return status([&] { element->value.as<TextBlock>().TextWrapping(enabled ? TextWrapping::Wrap : TextWrapping::NoWrap); }); }
int32_t zpd_winui3_text_set_alignment(zpd_winui3_element* element, int32_t alignment) { return status([&] { element->value.as<TextBlock>().TextAlignment(static_cast<TextAlignment>(alignment)); }); }

zpd_winui3_element* zpd_winui3_text_field_create(zpd_winui3_string value) { return create<zpd_winui3_element>([&] { TextBox box; box.Text(string_from(value)); return new TextBoxElement(box); }); }
int32_t zpd_winui3_text_field_set_value(zpd_winui3_element* element, zpd_winui3_string value) { return status([&] { static_cast<TextBoxElement*>(element)->text_box.Text(string_from(value)); }); }
int32_t zpd_winui3_text_field_get_value(const zpd_winui3_element* element, const void* data, zpd_winui3_string_fn receive) { return status([&] { with_utf8(static_cast<TextBoxElement const*>(element)->text_box.Text(), [&](zpd_winui3_string value) { receive(data, value); }); }); }
int32_t zpd_winui3_text_field_set_placeholder(zpd_winui3_element* element, zpd_winui3_string value) { return status([&] { static_cast<TextBoxElement*>(element)->text_box.PlaceholderText(string_from(value)); }); }
int32_t zpd_winui3_text_field_set_read_only(zpd_winui3_element* element, bool read_only) { return status([&] { static_cast<TextBoxElement*>(element)->text_box.IsReadOnly(read_only); }); }
int32_t zpd_winui3_text_field_clear_change_handler(zpd_winui3_element* element) { return status([&] { static_cast<TextBoxElement*>(element)->clear(); }); }
int32_t zpd_winui3_text_field_set_change_handler(zpd_winui3_element* element, const void* data, zpd_winui3_string_fn invoke, zpd_winui3_release_fn release) {
  return status([&] { auto box = static_cast<TextBoxElement*>(element); box->clear(); box->callback = Callback(data, release); box->change_token = box->text_box.TextChanged([box, invoke](auto&&, auto&&) { with_utf8(box->text_box.Text(), [&](zpd_winui3_string value) { invoke(box->callback.data, value); }); }); });
}

zpd_winui3_element* zpd_winui3_stack_panel_create(int32_t orientation, double spacing) { return create<zpd_winui3_element>([&] { StackPanel panel; panel.Orientation(static_cast<Orientation>(orientation)); panel.Spacing(spacing); return new zpd_winui3_element(panel); }); }
int32_t zpd_winui3_panel_append(zpd_winui3_element* panel, const zpd_winui3_element* child) { return status([&] { panel->value.as<Panel>().Children().Append(child->value); }); }
int32_t zpd_winui3_panel_clear(zpd_winui3_element* panel) { return status([&] { panel->value.as<Panel>().Children().Clear(); }); }

zpd_winui3_element* zpd_winui3_grid_create() { return create<zpd_winui3_element>([] { return new zpd_winui3_element(Grid()); }); }
int32_t zpd_winui3_grid_set_rows(zpd_winui3_element* element, const zpd_winui3_grid_length* rows, size_t length) { return status([&] { auto grid = element->value.as<Grid>(); grid.RowDefinitions().Clear(); for (size_t i = 0; i < length; ++i) { RowDefinition row; row.Height(grid_length(rows[i])); grid.RowDefinitions().Append(row); } }); }
int32_t zpd_winui3_grid_set_columns(zpd_winui3_element* element, const zpd_winui3_grid_length* columns, size_t length) { return status([&] { auto grid = element->value.as<Grid>(); grid.ColumnDefinitions().Clear(); for (size_t i = 0; i < length; ++i) { ColumnDefinition column; column.Width(grid_length(columns[i])); grid.ColumnDefinitions().Append(column); } }); }
int32_t zpd_winui3_grid_add(zpd_winui3_element* element, const zpd_winui3_element* child, int32_t row, int32_t column, int32_t row_span, int32_t column_span) { return status([&] { auto grid = element->value.as<Grid>(); Grid::SetRow(child->value, row); Grid::SetColumn(child->value, column); Grid::SetRowSpan(child->value, std::max(row_span, 1)); Grid::SetColumnSpan(child->value, std::max(column_span, 1)); grid.Children().Append(child->value); }); }

void zpd_winui3_element_release(zpd_winui3_element* element) { delete element; }
int32_t zpd_winui3_element_set_margin(zpd_winui3_element* element, zpd_winui3_thickness margin) { return status([&] { element->value.Margin({margin.left, margin.top, margin.right, margin.bottom}); }); }
int32_t zpd_winui3_element_set_width(zpd_winui3_element* element, double width) { return status([&] { element->value.Width(width); }); }
int32_t zpd_winui3_element_set_height(zpd_winui3_element* element, double height) { return status([&] { element->value.Height(height); }); }
int32_t zpd_winui3_element_set_horizontal_alignment(zpd_winui3_element* element, int32_t alignment) { return status([&] { element->value.HorizontalAlignment(static_cast<HorizontalAlignment>(alignment)); }); }
int32_t zpd_winui3_element_set_vertical_alignment(zpd_winui3_element* element, int32_t alignment) { return status([&] { element->value.VerticalAlignment(static_cast<VerticalAlignment>(alignment)); }); }

int32_t zpd_winui3_element_set_frame(zpd_winui3_element* element, double x, double y, double width, double height) {
  return status([&] {
    Canvas::SetLeft(element->value, x);
    Canvas::SetTop(element->value, y);
    element->value.Width(std::max(width, 0.0));
    element->value.Height(std::max(height, 0.0));
  });
}

int32_t zpd_winui3_element_set_context_menu(zpd_winui3_element* element, const zpd_winui3_menu_item* items, size_t length, const void* data, zpd_winui3_string_fn invoke, zpd_winui3_release_fn release) {
  auto target = std::make_shared<MenuTarget>();
  target->callback = Callback(data, release);
  target->invoke = invoke;
  return status([&] {
    MenuFlyout menu;
    append_menu_items(menu.Items(), target, items, length);
    // The flyout keeps the target alive for as long as it is attached.
    menu.Opening([target](auto&&, auto&&) {});
    element->value.ContextFlyout(menu);
  });
}

int32_t zpd_winui3_element_clear_context_menu(zpd_winui3_element* element) {
  return status([&] { element->value.ContextFlyout(nullptr); });
}

} // extern "C"
