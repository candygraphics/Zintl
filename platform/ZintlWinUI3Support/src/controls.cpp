#include "common.h"

#include <algorithm>
#include <cmath>
#include <vector>

#include <winrt/Windows.Globalization.NumberFormatting.h>

using namespace zpd;
using Windows::Foundation::IInspectable;

namespace {

/* Rust callback and the last values known on both sides of the boundary. */
struct ControlState {
  Callback callback;
  zpd_winui3_event_fn invoke{};
  std::wstring text;
  double number{NAN};
  bool flag{};
  int64_t index{-1};
  zpd_winui3_color color{0, 0, 0, 1};
  double x{};
  double y{};
  std::string navigation_id;
  /* An offset waiting until the scroll viewer can reach it. */
  bool scroll_pending{};

  /* Applies a pending offset once the extent allows it. */
  void apply_pending_scroll(ScrollViewer const& scroll) {
    if (!scroll_pending || !scroll.IsLoaded()) return;
    if (scroll.ScrollableWidth() < x - 0.5 || scroll.ScrollableHeight() < y - 0.5) return;
    scroll_pending = false;
    scroll.ChangeView(Windows::Foundation::IReference<double>(x), Windows::Foundation::IReference<double>(y), nullptr, true);
  }

  void emit(zpd_winui3_event const& event) const {
    if (invoke && callback.data) invoke(callback.data, &event);
  }

  void emit_text(int32_t kind, hstring const& value) {
    if (std::wstring_view(value) == text) return;
    text = value;
    with_utf8(value, [&](zpd_winui3_string utf8) {
      zpd_winui3_event event{};
      event.kind = kind;
      event.text = utf8;
      emit(event);
    });
  }

  void emit_index(int64_t value) {
    if (value < 0 || value == index) return;
    index = value;
    zpd_winui3_event event{};
    event.kind = ZPD_WINUI3_EVENT_SELECTION_CHANGED;
    event.index = value;
    emit(event);
  }

  void emit_number(double value) {
    if (std::isnan(value) || (!std::isnan(number) && std::abs(value - number) < 1e-9)) return;
    number = value;
    zpd_winui3_event event{};
    event.kind = ZPD_WINUI3_EVENT_VALUE_CHANGED;
    event.x = value;
    emit(event);
  }

  void emit_color(Windows::UI::Color value) {
    zpd_winui3_color next{value.R / 255.0, value.G / 255.0, value.B / 255.0, value.A / 255.0};
    auto same = [](double first, double second) { return std::abs(first - second) < 1.0 / 512.0; };
    if (same(next.red, color.red) && same(next.green, color.green) && same(next.blue, color.blue) && same(next.alpha, color.alpha)) return;
    color = next;
    zpd_winui3_event event{};
    event.kind = ZPD_WINUI3_EVENT_COLOR_CHANGED;
    event.color = next;
    emit(event);
  }
};

struct ControlElement final : zpd_winui3_element {
  int32_t kind;
  std::shared_ptr<ControlState> state = std::make_shared<ControlState>();
  /* The scroll viewer's document. */
  Canvas document{nullptr};
  /* The color button's swatch and flyout picker. */
  Border swatch{nullptr};
  ColorPicker picker{nullptr};
  /* Button content parts. */
  hstring title;
  hstring glyph;
  bool help{};
  std::vector<hstring> tab_titles;
  AutoSuggestBox search{nullptr};
  double minimum{0};
  double maximum{1};
  int32_t ticks{};
  bool snaps{};

  ControlElement(int32_t kind, FrameworkElement const& value) : zpd_winui3_element(value), kind(kind) {}

  bool is_button() const {
    return kind == ZPD_WINUI3_CONTROL_BUTTON || kind == ZPD_WINUI3_CONTROL_TOGGLE_BUTTON ||
           kind == ZPD_WINUI3_CONTROL_CHECK_BOX || kind == ZPD_WINUI3_CONTROL_RADIO_BUTTON;
  }
};

ControlElement* control(zpd_winui3_element* element) { return static_cast<ControlElement*>(element); }
ControlElement const* control(zpd_winui3_element const* element) { return static_cast<ControlElement const*>(element); }

std::weak_ptr<ControlState> weak(ControlElement const* element) { return element->state; }

Windows::UI::Color color_from(zpd_winui3_color color) {
  auto channel = [](double value) { return static_cast<uint8_t>(std::clamp(value, 0.0, 1.0) * 255.0 + 0.5); };
  return {channel(color.alpha), channel(color.red), channel(color.green), channel(color.blue)};
}

/* Removes `child` from its logical parent so it can be attached elsewhere. */
void detach(FrameworkElement const& child) {
  auto parent = child.Parent();
  if (!parent) return;
  if (auto panel = parent.try_as<Panel>()) {
    uint32_t index{};
    if (panel.Children().IndexOf(child, index)) panel.Children().RemoveAt(index);
  } else if (auto content = parent.try_as<ContentControl>()) {
    content.Content(nullptr);
  } else if (auto border = parent.try_as<Border>()) {
    border.Child(nullptr);
  }
}

void set_panel_children(Panel const& panel, const zpd_winui3_element* const* children, size_t length) {
  panel.Children().Clear();
  for (size_t index = 0; index < length; ++index) {
    detach(children[index]->value);
    panel.Children().Append(children[index]->value);
  }
}

void refresh_button_content(ControlElement* element) {
  auto button = element->value.as<ContentControl>();
  if (element->help) {
    button.Content(box_value(L"?"));
    return;
  }
  if (element->glyph.empty()) {
    button.Content(box_value(element->title));
    return;
  }
  FontIcon icon;
  icon.Glyph(element->glyph);
  icon.FontSize(16);
  if (element->title.empty()) {
    button.Content(icon);
    return;
  }
  StackPanel content;
  content.Orientation(Orientation::Horizontal);
  content.Spacing(8);
  TextBlock label;
  label.Text(element->title);
  content.Children().Append(icon);
  content.Children().Append(label);
  button.Content(content);
}

void refresh_slider_ticks(ControlElement* element) {
  auto slider = element->value.as<Slider>();
  double range = element->maximum - element->minimum;
  if (element->ticks > 1) {
    double frequency = range / (element->ticks - 1);
    slider.TickFrequency(frequency);
    slider.TickPlacement(Primitives::TickPlacement::Outside);
    slider.SnapsTo(element->snaps ? Primitives::SliderSnapsTo::Ticks : Primitives::SliderSnapsTo::StepValues);
  } else {
    slider.TickPlacement(Primitives::TickPlacement::None);
    slider.SnapsTo(Primitives::SliderSnapsTo::StepValues);
  }
  slider.StepFrequency(element->snaps && element->ticks > 1 ? range / (element->ticks - 1) : range / 1000.0);
}

void select_navigation_item(ControlElement* element) {
  auto navigation = element->value.as<NavigationView>();
  for (auto const& item : navigation.MenuItems()) {
    auto entry = item.try_as<NavigationViewItem>();
    if (!entry) continue;
    if (to_string(unbox_value_or<hstring>(entry.Tag(), L"")) == element->state->navigation_id) {
      navigation.SelectedItem(entry);
      return;
    }
  }
  navigation.SelectedItem(nullptr);
}

FrameworkElement make_control(ControlElement*& element, int32_t kind) {
  switch (kind) {
    case ZPD_WINUI3_CONTROL_CANVAS: {
      Canvas canvas;
      canvas.Background(Media::SolidColorBrush(Windows::UI::Colors::Transparent()));
      element = new ControlElement(kind, canvas);
      canvas.SizeChanged([state = weak(element)](IInspectable const&, SizeChangedEventArgs const& args) {
        auto locked = state.lock();
        if (!locked) return;
        zpd_winui3_event event{};
        event.kind = ZPD_WINUI3_EVENT_SIZE_CHANGED;
        event.x = args.NewSize().Width;
        event.y = args.NewSize().Height;
        locked->emit(event);
      });
      return canvas;
    }
    case ZPD_WINUI3_CONTROL_TEXT_BLOCK: {
      TextBlock text;
      text.TextTrimming(TextTrimming::CharacterEllipsis);
      text.VerticalAlignment(VerticalAlignment::Center);
      element = new ControlElement(kind, text);
      return text;
    }
    case ZPD_WINUI3_CONTROL_TEXT_BOX: {
      TextBox box;
      element = new ControlElement(kind, box);
      box.TextChanged([state = weak(element)](IInspectable const& sender, TextChangedEventArgs const&) {
        if (auto locked = state.lock()) locked->emit_text(ZPD_WINUI3_EVENT_TEXT_CHANGED, sender.as<TextBox>().Text());
      });
      return box;
    }
    case ZPD_WINUI3_CONTROL_BUTTON: {
      Button button;
      element = new ControlElement(kind, button);
      button.Click([state = weak(element)](IInspectable const&, RoutedEventArgs const&) {
        if (auto locked = state.lock()) {
          zpd_winui3_event event{};
          event.kind = ZPD_WINUI3_EVENT_CLICKED;
          locked->emit(event);
        }
      });
      return button;
    }
    case ZPD_WINUI3_CONTROL_TOGGLE_BUTTON:
    case ZPD_WINUI3_CONTROL_CHECK_BOX:
    case ZPD_WINUI3_CONTROL_RADIO_BUTTON: {
      Primitives::ToggleButton toggle{nullptr};
      if (kind == ZPD_WINUI3_CONTROL_TOGGLE_BUTTON) toggle = Primitives::ToggleButton();
      else if (kind == ZPD_WINUI3_CONTROL_CHECK_BOX) toggle = CheckBox();
      else toggle = RadioButton();
      element = new ControlElement(kind, toggle);
      // Click is raised only by the user, unlike Checked and Unchecked.
      toggle.Click([state = weak(element)](IInspectable const& sender, RoutedEventArgs const&) {
        auto locked = state.lock();
        if (!locked) return;
        locked->flag = unbox_value_or<bool>(sender.as<Primitives::ToggleButton>().IsChecked(), false);
        zpd_winui3_event event{};
        event.kind = ZPD_WINUI3_EVENT_STATE_CHANGED;
        event.flag = locked->flag;
        locked->emit(event);
      });
      return toggle;
    }
    case ZPD_WINUI3_CONTROL_TOGGLE_SWITCH: {
      ToggleSwitch toggle;
      toggle.OnContent(box_value(L""));
      toggle.OffContent(box_value(L""));
      element = new ControlElement(kind, toggle);
      toggle.Toggled([state = weak(element)](IInspectable const& sender, RoutedEventArgs const&) {
        auto locked = state.lock();
        if (!locked) return;
        bool on = sender.as<ToggleSwitch>().IsOn();
        if (on == locked->flag) return;
        locked->flag = on;
        zpd_winui3_event event{};
        event.kind = ZPD_WINUI3_EVENT_STATE_CHANGED;
        event.flag = on;
        locked->emit(event);
      });
      return toggle;
    }
    case ZPD_WINUI3_CONTROL_SLIDER: {
      Slider slider;
      element = new ControlElement(kind, slider);
      slider.ValueChanged([state = weak(element)](IInspectable const&, Primitives::RangeBaseValueChangedEventArgs const& args) {
        if (auto locked = state.lock()) locked->emit_number(args.NewValue());
      });
      return slider;
    }
    case ZPD_WINUI3_CONTROL_COMBO_BOX:
    case ZPD_WINUI3_CONTROL_EDITABLE_COMBO_BOX: {
      ComboBox combo;
      element = new ControlElement(kind, combo);
      if (kind == ZPD_WINUI3_CONTROL_EDITABLE_COMBO_BOX) {
        combo.IsEditable(true);
        combo.TextSubmitted([state = weak(element)](ComboBox const&, ComboBoxTextSubmittedEventArgs const& args) {
          if (auto locked = state.lock()) locked->emit_text(ZPD_WINUI3_EVENT_TEXT_CHANGED, args.Text());
        });
        combo.SelectionChanged([state = weak(element)](IInspectable const& sender, SelectionChangedEventArgs const&) {
          auto locked = state.lock();
          auto selected = sender.as<ComboBox>().SelectedItem();
          if (locked && selected) locked->emit_text(ZPD_WINUI3_EVENT_TEXT_CHANGED, unbox_value_or<hstring>(selected, L""));
        });
      } else {
        combo.SelectionChanged([state = weak(element)](IInspectable const& sender, SelectionChangedEventArgs const&) {
          if (auto locked = state.lock()) locked->emit_index(sender.as<ComboBox>().SelectedIndex());
        });
      }
      return combo;
    }
    case ZPD_WINUI3_CONTROL_SELECTOR_BAR: {
      SelectorBar bar;
      element = new ControlElement(kind, bar);
      bar.SelectionChanged([state = weak(element)](SelectorBar const& sender, SelectorBarSelectionChangedEventArgs const&) {
        auto locked = state.lock();
        auto selected = sender.SelectedItem();
        if (!locked || !selected) return;
        uint32_t index{};
        if (sender.Items().IndexOf(selected, index)) locked->emit_index(index);
      });
      return bar;
    }
    case ZPD_WINUI3_CONTROL_NUMBER_BOX: {
      NumberBox box;
      box.SpinButtonPlacementMode(NumberBoxSpinButtonPlacementMode::Compact);
      element = new ControlElement(kind, box);
      box.ValueChanged([state = weak(element)](NumberBox const&, NumberBoxValueChangedEventArgs const& args) {
        if (auto locked = state.lock()) locked->emit_number(args.NewValue());
      });
      return box;
    }
    case ZPD_WINUI3_CONTROL_AUTO_SUGGEST_BOX: {
      AutoSuggestBox box;
      box.QueryIcon(SymbolIcon(Symbol::Find));
      element = new ControlElement(kind, box);
      box.TextChanged([state = weak(element)](AutoSuggestBox const& sender, AutoSuggestBoxTextChangedEventArgs const& args) {
        auto locked = state.lock();
        if (locked && args.Reason() == AutoSuggestionBoxTextChangeReason::UserInput) {
          locked->emit_text(ZPD_WINUI3_EVENT_TEXT_CHANGED, sender.Text());
        }
      });
      return box;
    }
    case ZPD_WINUI3_CONTROL_COLOR_BUTTON:
    case ZPD_WINUI3_CONTROL_COLOR_PICKER: {
      ColorPicker picker;
      picker.IsAlphaEnabled(false);
      FrameworkElement root{nullptr};
      Border swatch{nullptr};
      if (kind == ZPD_WINUI3_CONTROL_COLOR_BUTTON) {
        Button button;
        button.Padding({4, 4, 4, 4});
        button.HorizontalContentAlignment(HorizontalAlignment::Stretch);
        button.VerticalContentAlignment(VerticalAlignment::Stretch);
        swatch = Border();
        swatch.CornerRadius({3, 3, 3, 3});
        button.Content(swatch);
        Flyout flyout;
        flyout.Content(picker);
        button.Flyout(flyout);
        root = button;
      } else {
        root = picker;
      }
      element = new ControlElement(kind, root);
      element->picker = picker;
      element->swatch = swatch;
      picker.ColorChanged([state = weak(element), swatch](ColorPicker const&, ColorChangedEventArgs const& args) {
        if (swatch) swatch.Background(Media::SolidColorBrush(args.NewColor()));
        if (auto locked = state.lock()) locked->emit_color(args.NewColor());
      });
      return root;
    }
    case ZPD_WINUI3_CONTROL_SCROLL_VIEWER: {
      ScrollViewer scroll;
      Canvas document;
      document.Background(Media::SolidColorBrush(Windows::UI::Colors::Transparent()));
      scroll.Content(document);
      element = new ControlElement(kind, scroll);
      element->document = document;
      // A restored offset waits until the viewer is loaded and measured.
      scroll.Loaded([state = weak(element)](IInspectable const& sender, RoutedEventArgs const&) {
        if (auto locked = state.lock()) locked->apply_pending_scroll(sender.as<ScrollViewer>());
      });
      document.SizeChanged([state = weak(element), scroll = make_weak(scroll)](IInspectable const&, SizeChangedEventArgs const&) {
        auto locked = state.lock();
        auto viewer = scroll.get();
        if (locked && viewer) locked->apply_pending_scroll(viewer);
      });
      scroll.ViewChanged([state = weak(element)](IInspectable const& sender, ScrollViewerViewChangedEventArgs const& args) {
        auto locked = state.lock();
        if (!locked || args.IsIntermediate()) return;
        auto scroll = sender.as<ScrollViewer>();
        double x = scroll.HorizontalOffset();
        double y = scroll.VerticalOffset();
        if (std::abs(x - locked->x) < 0.5 && std::abs(y - locked->y) < 0.5) return;
        locked->x = x;
        locked->y = y;
        zpd_winui3_event event{};
        event.kind = ZPD_WINUI3_EVENT_SCROLLED;
        event.x = x;
        event.y = y;
        locked->emit(event);
      });
      return scroll;
    }
    case ZPD_WINUI3_CONTROL_TAB_VIEW: {
      TabView tabs;
      tabs.IsAddTabButtonVisible(false);
      tabs.CanReorderTabs(false);
      tabs.CanDragTabs(false);
      element = new ControlElement(kind, tabs);
      tabs.SelectionChanged([state = weak(element)](IInspectable const& sender, SelectionChangedEventArgs const&) {
        if (auto locked = state.lock()) locked->emit_index(sender.as<TabView>().SelectedIndex());
      });
      return tabs;
    }
    case ZPD_WINUI3_CONTROL_NAVIGATION_VIEW: {
      NavigationView navigation;
      navigation.PaneDisplayMode(NavigationViewPaneDisplayMode::Left);
      navigation.IsBackButtonVisible(NavigationViewBackButtonVisible::Collapsed);
      navigation.IsSettingsVisible(false);
      navigation.OpenPaneLength(240);
      element = new ControlElement(kind, navigation);
      navigation.SelectionChanged([state = weak(element)](NavigationView const&, NavigationViewSelectionChangedEventArgs const& args) {
        auto locked = state.lock();
        auto container = args.SelectedItemContainer();
        if (!locked || !container) return;
        auto id = to_string(unbox_value_or<hstring>(container.Tag(), L""));
        if (id.empty() || id == locked->navigation_id) return;
        locked->navigation_id = id;
        zpd_winui3_event event{};
        event.kind = ZPD_WINUI3_EVENT_NAVIGATION_SELECTED;
        event.text = {reinterpret_cast<uint8_t const*>(id.data()), id.size()};
        locked->emit(event);
      });
      return navigation;
    }
    default:
      throw hresult_invalid_argument(L"unknown control kind");
  }
}

}  // namespace

extern "C" {

zpd_winui3_element* zpd_winui3_control_create(int32_t kind) {
  return create<zpd_winui3_element>([&] {
    ControlElement* element{};
    auto value = make_control(element, kind);
    // Frames come from the Rust layout, so templates must not impose minimums.
    value.MinWidth(0);
    value.MinHeight(0);
    return element;
  });
}

int32_t zpd_winui3_control_set_text(zpd_winui3_element* target, int32_t property, zpd_winui3_string utf8) {
  return status([&] {
    auto element = control(target);
    auto value = string_from(utf8);
    if (property == ZPD_WINUI3_TEXT_GLYPH && element->is_button()) {
      element->glyph = value;
      refresh_button_content(element);
      return;
    }
    if (property == ZPD_WINUI3_TEXT_PLACEHOLDER) {
      switch (element->kind) {
        case ZPD_WINUI3_CONTROL_TEXT_BOX: element->value.as<TextBox>().PlaceholderText(value); break;
        case ZPD_WINUI3_CONTROL_COMBO_BOX:
        case ZPD_WINUI3_CONTROL_EDITABLE_COMBO_BOX: element->value.as<ComboBox>().PlaceholderText(value); break;
        case ZPD_WINUI3_CONTROL_NUMBER_BOX: element->value.as<NumberBox>().PlaceholderText(value); break;
        case ZPD_WINUI3_CONTROL_AUTO_SUGGEST_BOX: element->value.as<AutoSuggestBox>().PlaceholderText(value); break;
        default: break;
      }
      return;
    }
    if (property != ZPD_WINUI3_TEXT) return;
    if (element->is_button()) {
      element->title = value;
      refresh_button_content(element);
      return;
    }
    switch (element->kind) {
      case ZPD_WINUI3_CONTROL_TEXT_BLOCK: element->value.as<TextBlock>().Text(value); break;
      case ZPD_WINUI3_CONTROL_TEXT_BOX:
        element->state->text = value;
        if (element->value.as<TextBox>().Text() != value) element->value.as<TextBox>().Text(value);
        break;
      case ZPD_WINUI3_CONTROL_EDITABLE_COMBO_BOX:
        element->state->text = value;
        if (element->value.as<ComboBox>().Text() != value) element->value.as<ComboBox>().Text(value);
        break;
      case ZPD_WINUI3_CONTROL_AUTO_SUGGEST_BOX:
        element->state->text = value;
        if (element->value.as<AutoSuggestBox>().Text() != value) element->value.as<AutoSuggestBox>().Text(value);
        break;
      default: break;
    }
  });
}

int32_t zpd_winui3_control_get_text(const zpd_winui3_element* target, int32_t property, const void* data, zpd_winui3_string_fn receive) {
  return status([&] {
    auto element = control(target);
    hstring value;
    if (property == ZPD_WINUI3_TEXT) {
      switch (element->kind) {
        case ZPD_WINUI3_CONTROL_TEXT_BLOCK: value = element->value.as<TextBlock>().Text(); break;
        case ZPD_WINUI3_CONTROL_TEXT_BOX: value = element->value.as<TextBox>().Text(); break;
        case ZPD_WINUI3_CONTROL_EDITABLE_COMBO_BOX: value = element->value.as<ComboBox>().Text(); break;
        case ZPD_WINUI3_CONTROL_AUTO_SUGGEST_BOX: value = element->value.as<AutoSuggestBox>().Text(); break;
        default: value = element->title; break;
      }
    }
    with_utf8(value, [&](zpd_winui3_string utf8) { receive(data, utf8); });
  });
}

int32_t zpd_winui3_control_set_number(zpd_winui3_element* target, int32_t property, double value) {
  return status([&] {
    auto element = control(target);
    switch (element->kind) {
      case ZPD_WINUI3_CONTROL_SLIDER: {
        auto slider = element->value.as<Slider>();
        if (property == ZPD_WINUI3_NUMBER_MINIMUM) {
          element->minimum = value;
          slider.Minimum(value);
          refresh_slider_ticks(element);
        } else if (property == ZPD_WINUI3_NUMBER_MAXIMUM) {
          element->maximum = value;
          slider.Maximum(value);
          refresh_slider_ticks(element);
        } else if (property == ZPD_WINUI3_NUMBER_TICK_COUNT) {
          element->ticks = static_cast<int32_t>(value);
          refresh_slider_ticks(element);
        } else if (property == ZPD_WINUI3_NUMBER_VALUE) {
          element->state->number = value;
          slider.Value(value);
        }
        break;
      }
      case ZPD_WINUI3_CONTROL_NUMBER_BOX: {
        auto box = element->value.as<NumberBox>();
        if (property == ZPD_WINUI3_NUMBER_MINIMUM) box.Minimum(value);
        else if (property == ZPD_WINUI3_NUMBER_MAXIMUM) box.Maximum(value);
        else if (property == ZPD_WINUI3_NUMBER_STEP) {
          box.SmallChange(value);
          box.LargeChange(value * 10);
        } else if (property == ZPD_WINUI3_NUMBER_FRACTION_DIGITS) {
          Windows::Globalization::NumberFormatting::DecimalFormatter formatter;
          formatter.IntegerDigits(1);
          formatter.FractionDigits(static_cast<int32_t>(value));
          box.NumberFormatter(formatter);
        } else if (property == ZPD_WINUI3_NUMBER_VALUE) {
          element->state->number = value;
          box.Value(value);
        }
        break;
      }
      case ZPD_WINUI3_CONTROL_COMBO_BOX:
        if (property == ZPD_WINUI3_NUMBER_SELECTED_INDEX) {
          element->state->index = static_cast<int64_t>(value);
          auto combo = element->value.as<ComboBox>();
          if (value < combo.Items().Size()) combo.SelectedIndex(static_cast<int32_t>(value));
        }
        break;
      case ZPD_WINUI3_CONTROL_SELECTOR_BAR:
        if (property == ZPD_WINUI3_NUMBER_SELECTED_INDEX) {
          auto bar = element->value.as<SelectorBar>();
          auto index = static_cast<int64_t>(value);
          element->state->index = index;
          bar.SelectedItem(index >= 0 && index < bar.Items().Size() ? bar.Items().GetAt(static_cast<uint32_t>(index)) : nullptr);
        }
        break;
      case ZPD_WINUI3_CONTROL_TAB_VIEW:
        if (property == ZPD_WINUI3_NUMBER_SELECTED_INDEX) {
          element->state->index = static_cast<int64_t>(value);
          auto tabs = element->value.as<TabView>();
          if (value >= 0 && value < tabs.TabItems().Size()) tabs.SelectedIndex(static_cast<int32_t>(value));
        }
        break;
      default: break;
    }
  });
}

int32_t zpd_winui3_control_get_number(const zpd_winui3_element* target, int32_t property, double* value) {
  return status([&] {
    auto element = control(target);
    if (property == ZPD_WINUI3_NUMBER_VALUE) {
      if (element->kind == ZPD_WINUI3_CONTROL_SLIDER) *value = element->value.as<Slider>().Value();
      else if (element->kind == ZPD_WINUI3_CONTROL_NUMBER_BOX) *value = element->value.as<NumberBox>().Value();
      else *value = NAN;
    } else if (property == ZPD_WINUI3_NUMBER_SELECTED_INDEX) {
      *value = static_cast<double>(element->state->index);
    } else {
      *value = NAN;
    }
  });
}

int32_t zpd_winui3_control_set_bool(zpd_winui3_element* target, int32_t property, bool value) {
  return status([&] {
    auto element = control(target);
    switch (property) {
      case ZPD_WINUI3_BOOL_CHECKED:
        element->state->flag = value;
        if (element->kind == ZPD_WINUI3_CONTROL_TOGGLE_SWITCH) element->value.as<ToggleSwitch>().IsOn(value);
        else if (element->is_button() && element->kind != ZPD_WINUI3_CONTROL_BUTTON) element->value.as<Primitives::ToggleButton>().IsChecked(box_value(value).as<Windows::Foundation::IReference<bool>>());
        break;
      case ZPD_WINUI3_BOOL_ENABLED:
        if (auto control = element->value.try_as<Control>()) control.IsEnabled(value);
        else element->value.Opacity(value ? 1.0 : 0.4);
        break;
      case ZPD_WINUI3_BOOL_READ_ONLY:
        if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BOX) element->value.as<TextBox>().IsReadOnly(value);
        break;
      case ZPD_WINUI3_BOOL_MULTILINE:
        if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BOX) {
          element->value.as<TextBox>().AcceptsReturn(value);
          element->value.as<TextBox>().TextWrapping(value ? TextWrapping::Wrap : TextWrapping::NoWrap);
        } else if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BLOCK) {
          element->value.as<TextBlock>().TextWrapping(value ? TextWrapping::Wrap : TextWrapping::NoWrap);
        }
        break;
      case ZPD_WINUI3_BOOL_BORDERLESS:
        if (element->is_button()) {
          auto button = element->value.as<Control>();
          if (value) {
            button.Background(Media::SolidColorBrush(Windows::UI::Colors::Transparent()));
            button.BorderThickness({0, 0, 0, 0});
          } else {
            button.ClearValue(Control::BackgroundProperty());
            button.ClearValue(Control::BorderThicknessProperty());
          }
        }
        break;
      case ZPD_WINUI3_BOOL_DESTRUCTIVE:
        if (auto control = element->value.try_as<Control>()) {
          if (value) control.Foreground(brush_from({0.77, 0.17, 0.11, 1.0}));
          else control.ClearValue(Control::ForegroundProperty());
        }
        break;
      case ZPD_WINUI3_BOOL_SNAP_TO_TICKS:
        if (element->kind == ZPD_WINUI3_CONTROL_SLIDER) {
          element->snaps = value;
          refresh_slider_ticks(element);
        }
        break;
      case ZPD_WINUI3_BOOL_HORIZONTAL_SCROLL:
        if (element->kind == ZPD_WINUI3_CONTROL_SCROLL_VIEWER) {
          auto scroll = element->value.as<ScrollViewer>();
          scroll.HorizontalScrollMode(value ? ScrollMode::Auto : ScrollMode::Disabled);
          scroll.HorizontalScrollBarVisibility(value ? ScrollBarVisibility::Auto : ScrollBarVisibility::Disabled);
        }
        break;
      case ZPD_WINUI3_BOOL_VERTICAL_SCROLL:
        if (element->kind == ZPD_WINUI3_CONTROL_SCROLL_VIEWER) {
          auto scroll = element->value.as<ScrollViewer>();
          scroll.VerticalScrollMode(value ? ScrollMode::Auto : ScrollMode::Disabled);
          scroll.VerticalScrollBarVisibility(value ? ScrollBarVisibility::Auto : ScrollBarVisibility::Disabled);
        }
        break;
      case ZPD_WINUI3_BOOL_HELP:
        if (element->is_button()) {
          element->help = value;
          element->value.as<Control>().CornerRadius(value ? CornerRadius{16, 16, 16, 16} : CornerRadius{4, 4, 4, 4});
          refresh_button_content(element);
        }
        break;
      case ZPD_WINUI3_BOOL_DEFAULT_ACTION:
      case ZPD_WINUI3_BOOL_CANCEL_ACTION:
        if (element->kind == ZPD_WINUI3_CONTROL_BUTTON) {
          auto button = element->value.as<Button>();
          bool is_default = property == ZPD_WINUI3_BOOL_DEFAULT_ACTION;
          if (is_default) {
            if (value) {
              auto style = Application::Current().Resources().TryLookup(box_value(L"AccentButtonStyle"));
              if (style) button.Style(style.as<Style>());
            } else {
              button.ClearValue(FrameworkElement::StyleProperty());
            }
          }
          auto key = is_default ? Windows::System::VirtualKey::Enter : Windows::System::VirtualKey::Escape;
          auto accelerators = button.KeyboardAccelerators();
          for (uint32_t index = accelerators.Size(); index > 0; --index) {
            if (accelerators.GetAt(index - 1).Key() == key) accelerators.RemoveAt(index - 1);
          }
          if (value) {
            Input::KeyboardAccelerator accelerator;
            accelerator.Key(key);
            accelerators.Append(accelerator);
          }
        }
        break;
      case ZPD_WINUI3_BOOL_ALPHA:
        if (element->picker) element->picker.IsAlphaEnabled(value);
        break;
      default: break;
    }
  });
}

int32_t zpd_winui3_control_get_bool(const zpd_winui3_element* target, int32_t property, bool* value) {
  return status([&] {
    auto element = control(target);
    *value = property == ZPD_WINUI3_BOOL_CHECKED && element->state->flag;
  });
}

int32_t zpd_winui3_control_set_color(zpd_winui3_element* target, zpd_winui3_color value) {
  return status([&] {
    auto element = control(target);
    if (!element->picker) return;
    element->state->color = value;
    element->picker.Color(color_from(value));
    if (element->swatch) element->swatch.Background(brush_from(value));
  });
}

int32_t zpd_winui3_control_get_color(const zpd_winui3_element* target, zpd_winui3_color* value) {
  return status([&] { *value = control(target)->state->color; });
}

int32_t zpd_winui3_control_set_items(zpd_winui3_element* target, const zpd_winui3_string* items, size_t length) {
  return status([&] {
    auto element = control(target);
    switch (element->kind) {
      case ZPD_WINUI3_CONTROL_COMBO_BOX:
      case ZPD_WINUI3_CONTROL_EDITABLE_COMBO_BOX: {
        auto combo = element->value.as<ComboBox>();
        auto text = element->kind == ZPD_WINUI3_CONTROL_EDITABLE_COMBO_BOX ? combo.Text() : hstring();
        combo.Items().Clear();
        for (size_t index = 0; index < length; ++index) combo.Items().Append(box_value(string_from(items[index])));
        auto selected = element->state->index;
        if (element->kind == ZPD_WINUI3_CONTROL_COMBO_BOX) combo.SelectedIndex(selected < static_cast<int64_t>(length) ? static_cast<int32_t>(selected) : -1);
        else combo.Text(text);
        break;
      }
      case ZPD_WINUI3_CONTROL_SELECTOR_BAR: {
        auto bar = element->value.as<SelectorBar>();
        bar.Items().Clear();
        for (size_t index = 0; index < length; ++index) {
          SelectorBarItem item;
          item.Text(string_from(items[index]));
          bar.Items().Append(item);
        }
        auto selected = element->state->index;
        if (selected >= 0 && selected < static_cast<int64_t>(length)) bar.SelectedItem(bar.Items().GetAt(static_cast<uint32_t>(selected)));
        break;
      }
      case ZPD_WINUI3_CONTROL_TAB_VIEW: {
        element->tab_titles.clear();
        for (size_t index = 0; index < length; ++index) element->tab_titles.push_back(string_from(items[index]));
        auto tabs = element->value.as<TabView>().TabItems();
        for (uint32_t index = 0; index < tabs.Size() && index < length; ++index) {
          tabs.GetAt(index).as<TabViewItem>().Header(box_value(element->tab_titles[index]));
        }
        break;
      }
      default: break;
    }
  });
}

int32_t zpd_winui3_control_set_children(zpd_winui3_element* target, const zpd_winui3_element* const* children, size_t length) {
  return status([&] {
    auto element = control(target);
    switch (element->kind) {
      case ZPD_WINUI3_CONTROL_CANVAS: set_panel_children(element->value.as<Panel>(), children, length); break;
      case ZPD_WINUI3_CONTROL_SCROLL_VIEWER: set_panel_children(element->document, children, length); break;
      case ZPD_WINUI3_CONTROL_TAB_VIEW: {
        auto tabs = element->value.as<TabView>();
        for (auto const& item : tabs.TabItems()) item.as<TabViewItem>().Content(nullptr);
        tabs.TabItems().Clear();
        for (size_t index = 0; index < length; ++index) {
          detach(children[index]->value);
          TabViewItem item;
          item.IsClosable(false);
          item.Header(box_value(index < element->tab_titles.size() ? element->tab_titles[index] : hstring()));
          item.Content(children[index]->value);
          tabs.TabItems().Append(item);
        }
        if (element->state->index >= 0 && element->state->index < static_cast<int64_t>(length)) tabs.SelectedIndex(static_cast<int32_t>(element->state->index));
        break;
      }
      case ZPD_WINUI3_CONTROL_NAVIGATION_VIEW: {
        auto navigation = element->value.as<NavigationView>();
        navigation.Content(nullptr);
        if (length) {
          detach(children[0]->value);
          navigation.Content(children[0]->value);
        }
        break;
      }
      default: throw hresult_invalid_argument(L"the control has no children");
    }
  });
}

int32_t zpd_winui3_control_set_event_handler(zpd_winui3_element* target, const void* data, zpd_winui3_event_fn invoke, zpd_winui3_release_fn release) {
  auto element = control(target);
  return status([&] {
    element->state->invoke = nullptr;
    element->state->callback = Callback(data, release);
    element->state->invoke = invoke;
  });
}

int32_t zpd_winui3_scroll_viewer_set_extent(zpd_winui3_element* target, double width, double height) {
  return status([&] {
    auto element = control(target);
    element->document.Width(std::max(width, 0.0));
    element->document.Height(std::max(height, 0.0));
  });
}

int32_t zpd_winui3_scroll_viewer_scroll_to(zpd_winui3_element* target, double x, double y) {
  return status([&] {
    auto element = control(target);
    element->state->x = x;
    element->state->y = y;
    element->state->scroll_pending = true;
    element->state->apply_pending_scroll(element->value.as<ScrollViewer>());
  });
}

int32_t zpd_winui3_navigation_set_items(zpd_winui3_element* target, const zpd_winui3_navigation_item* items, size_t length) {
  return status([&] {
    auto element = control(target);
    auto navigation = element->value.as<NavigationView>();
    navigation.MenuItems().Clear();
    for (size_t index = 0; index < length; ++index) {
      auto const& source = items[index];
      if (source.header) {
        NavigationViewItemHeader header;
        header.Content(box_value(string_from(source.title)));
        navigation.MenuItems().Append(header);
        continue;
      }
      NavigationViewItem item;
      item.Content(box_value(string_from(source.title)));
      item.Tag(box_value(string_from(source.id)));
      if (source.glyph.length) {
        FontIcon icon;
        icon.Glyph(string_from(source.glyph));
        item.Icon(icon);
      }
      navigation.MenuItems().Append(item);
    }
    select_navigation_item(element);
  });
}

int32_t zpd_winui3_navigation_select(zpd_winui3_element* target, zpd_winui3_string id) {
  return status([&] {
    auto element = control(target);
    element->state->navigation_id = std::string(bytes(id));
    select_navigation_item(element);
  });
}

int32_t zpd_winui3_navigation_set_search(zpd_winui3_element* target, bool enabled, zpd_winui3_string value, zpd_winui3_string placeholder) {
  return status([&] {
    auto element = control(target);
    auto navigation = element->value.as<NavigationView>();
    if (!enabled) {
      element->search = nullptr;
      navigation.AutoSuggestBox(nullptr);
      return;
    }
    if (!element->search) {
      AutoSuggestBox box;
      box.QueryIcon(SymbolIcon(Symbol::Find));
      box.TextChanged([state = weak(element)](AutoSuggestBox const& sender, AutoSuggestBoxTextChangedEventArgs const& args) {
        auto locked = state.lock();
        if (locked && args.Reason() == AutoSuggestionBoxTextChangeReason::UserInput) {
          locked->emit_text(ZPD_WINUI3_EVENT_SEARCH_CHANGED, sender.Text());
        }
      });
      element->search = box;
      navigation.AutoSuggestBox(box);
    }
    auto text = string_from(value);
    element->state->text = text;
    if (element->search.Text() != text) element->search.Text(text);
    element->search.PlaceholderText(string_from(placeholder));
  });
}

}  // extern "C"
