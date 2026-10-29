#pragma once

#include "zpd_winui3.h"

#define NOMINMAX
#include <windows.h>
#undef GetCurrentTime

#include <memory>
#include <string>
#include <string_view>
#include <utility>

#include <winrt/Microsoft.UI.Dispatching.h>
#include <winrt/Microsoft.UI.Windowing.h>
#include <winrt/Microsoft.UI.Xaml.Controls.Primitives.h>
#include <winrt/Microsoft.UI.Xaml.Controls.h>
#include <winrt/Microsoft.UI.Xaml.Input.h>
#include <winrt/Microsoft.UI.Xaml.Media.h>
#include <winrt/Microsoft.UI.Xaml.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.UI.h>

namespace zpd {

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;

extern thread_local std::string last_error;

inline std::string_view bytes(zpd_winui3_string value) {
  return {reinterpret_cast<const char*>(value.data), value.length};
}

inline hstring string_from(zpd_winui3_string value) { return to_hstring(bytes(value)); }

/* Passes a UTF-8 copy of `value` to `body` as a borrowed zpd_winui3_string. */
template <typename F>
void with_utf8(hstring const& value, F&& body) {
  auto utf8 = to_string(value);
  body(zpd_winui3_string{reinterpret_cast<uint8_t const*>(utf8.data()), utf8.size()});
}

template <typename F>
int32_t status(F&& body) noexcept {
  try {
    body();
    last_error.clear();
    return 0;
  } catch (hresult_error const& error) {
    last_error = to_string(error.message());
    return error.code().value;
  } catch (std::exception const& error) {
    last_error = error.what();
    return E_FAIL;
  } catch (...) {
    last_error = "unknown native exception";
    return E_FAIL;
  }
}

template <typename T, typename F>
T* create(F&& body) noexcept {
  try {
    auto result = body();
    last_error.clear();
    return result;
  } catch (hresult_error const& error) {
    last_error = to_string(error.message());
  } catch (std::exception const& error) {
    last_error = error.what();
  } catch (...) {
    last_error = "unknown native exception";
  }
  return nullptr;
}

/* Owns Rust callback data and releases it exactly once. */
struct Callback {
  const void* data{};
  zpd_winui3_release_fn release{};

  Callback() = default;
  Callback(const void* data, zpd_winui3_release_fn release) : data(data), release(release) {}
  Callback(Callback const&) = delete;
  Callback& operator=(Callback const&) = delete;
  Callback(Callback&& other) noexcept
      : data(std::exchange(other.data, nullptr)), release(std::exchange(other.release, nullptr)) {}
  Callback& operator=(Callback&& other) noexcept {
    if (this != &other) {
      reset();
      data = std::exchange(other.data, nullptr);
      release = std::exchange(other.release, nullptr);
    }
    return *this;
  }
  ~Callback() { reset(); }
  void reset() {
    if (data && release) release(data);
    data = nullptr;
    release = nullptr;
  }
};

/* Receives the ids of chosen menu items; menus hold it weakly. */
struct MenuTarget {
  Callback callback;
  zpd_winui3_string_fn invoke{};

  void select(std::string const& id) const {
    if (!invoke || !callback.data) return;
    invoke(callback.data, zpd_winui3_string{reinterpret_cast<uint8_t const*>(id.data()), id.size()});
  }
};

void append_menu_items(
    Windows::Foundation::Collections::IVector<MenuFlyoutItemBase> const& target,
    std::shared_ptr<MenuTarget> const& menu_target,
    const zpd_winui3_menu_item* items,
    size_t length);

Media::SolidColorBrush brush_from(zpd_winui3_color color);

}  // namespace zpd

struct zpd_winui3_element {
  winrt::Microsoft::UI::Xaml::FrameworkElement value{nullptr};
  explicit zpd_winui3_element(winrt::Microsoft::UI::Xaml::FrameworkElement const& value)
      : value(value) {}
  virtual ~zpd_winui3_element() = default;
};

struct zpd_winui3_window {
  winrt::Microsoft::UI::Xaml::Window window{nullptr};
  winrt::Microsoft::UI::Xaml::Controls::Grid shell{nullptr};
  winrt::Microsoft::UI::Xaml::Controls::MenuBar menu_bar{nullptr};
  winrt::Microsoft::UI::Xaml::FrameworkElement content{nullptr};
  std::shared_ptr<zpd::MenuTarget> menu_target;
  zpd::Callback closed_callback;
  zpd_winui3_invoke_fn closed_invoke{};
  winrt::event_token closed_token{};
  /* Open dialogs and file pickers. */
  std::shared_ptr<int> sheets = std::make_shared<int>(0);
  bool closed{};

  explicit zpd_winui3_window(int32_t style);
  HWND hwnd() const;
  winrt::Microsoft::UI::Xaml::XamlRoot xaml_root() const;
};
