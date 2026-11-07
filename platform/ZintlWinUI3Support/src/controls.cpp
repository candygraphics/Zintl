#include "common.h"

#include <algorithm>
#include <cmath>
#include <cwchar>
#include <unordered_map>
#include <vector>

#include <winrt/Microsoft.UI.Text.h>
#include <winrt/Microsoft.UI.Xaml.Automation.h>
#include <winrt/Microsoft.UI.Xaml.Shapes.h>
#include <winrt/Windows.Globalization.NumberFormatting.h>
#include <winrt/Windows.System.h>
#include <winrt/Windows.UI.Text.h>

using namespace zpd;
using Windows::Foundation::IInspectable;
using Windows::Foundation::IReference;
namespace NumberFormatting = Windows::Globalization::NumberFormatting;

namespace {

/* Horizontal margin and padding of list rows; together ZPD_WINUI3_LIST_ROW_INSET on both sides. */
constexpr double LIST_ROW_MARGIN = 4;
constexpr double LIST_ROW_PADDING = 8;
static_assert(2 * (LIST_ROW_MARGIN + LIST_ROW_PADDING) == ZPD_WINUI3_LIST_ROW_INSET);

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
  /* The selected list row id. */
  std::string list_id;
  bool shows_menu{};
  bool keeps_state{};
  /* Text last reported as committed. */
  std::wstring committed;
  /* Number box formatting; `display` replaces the formatted value when set. */
  int32_t fraction_digits{2};
  std::wstring display;
  /* Set while the document is replaced, so no change is reported. */
  bool replacing{};

  /* Applies a pending offset once the extent allows it. */
  void apply_pending_scroll(ScrollViewer const& scroll) {
    if (!scroll_pending || !scroll.IsLoaded()) return;
    if (scroll.ScrollableWidth() < x - 0.5 || scroll.ScrollableHeight() < y - 0.5) return;
    scroll_pending = false;
    scroll.ChangeView(IReference<double>(x), IReference<double>(y), nullptr, true);
  }

  void emit(zpd_winui3_event const& event) const {
    if (invoke && callback.data) invoke(callback.data, &event);
  }

  void emit_string(int32_t kind, std::string const& value, bool flag = false) const {
    zpd_winui3_event event{};
    event.kind = kind;
    event.flag = flag;
    event.text = {reinterpret_cast<uint8_t const*>(value.data()), value.size()};
    emit(event);
  }

  void emit_text(int32_t kind, hstring const& value) {
    if (std::wstring_view(value) == text) return;
    text = value;
    emit_string(kind, to_string(value));
  }

  /* Reports the final text when editing ends; a focus change repeats no unchanged commit. */
  void emit_commit(hstring const& value, bool repeat) {
    if (!repeat && std::wstring_view(value) == committed) return;
    committed = value;
    emit_string(ZPD_WINUI3_EVENT_TEXT_COMMITTED, to_string(value));
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

  void emit_rich(RichEditBox const& box) const {
    if (replacing) return;
    read_rich_text(box, [&](zpd_winui3_rich_text const& rich) {
      zpd_winui3_event event{};
      event.kind = ZPD_WINUI3_EVENT_RICH_TEXT_CHANGED;
      event.rich = &rich;
      emit(event);
    });
  }

  std::wstring format(double value) const {
    if (!display.empty() && !std::isnan(number) && std::abs(value - number) < 1e-9) return display;
    wchar_t buffer[64]{};
    std::swprintf(buffer, std::size(buffer), L"%.*f", std::clamp(fraction_digits, 0, 15), value);
    return buffer;
  }
};

/* Formats a number box's value, showing the declared text, and parses typed text. */
struct NumberFormatter : implements<NumberFormatter, NumberFormatting::INumberFormatter2, NumberFormatting::INumberParser> {
  std::weak_ptr<ControlState> state;

  explicit NumberFormatter(std::weak_ptr<ControlState> state) : state(std::move(state)) {}

  hstring FormatInt(int64_t value) const { return FormatDouble(static_cast<double>(value)); }
  hstring FormatUInt(uint64_t value) const { return FormatDouble(static_cast<double>(value)); }
  hstring FormatDouble(double value) const {
    auto locked = state.lock();
    return locked ? hstring(locked->format(value)) : hstring(std::to_wstring(value));
  }

  IReference<double> ParseDouble(hstring const& text) const {
    auto locked = state.lock();
    if (!locked) return nullptr;
    // Declared text is the view's to parse, so typed text is reported and the value kept.
    if (!locked->display.empty()) {
      if (std::wstring_view(text) != locked->display) locked->emit_commit(text, true);
      return std::isnan(locked->number) ? nullptr : IReference<double>(locked->number);
    }
    std::wstring value(text);
    wchar_t* end{};
    double parsed = std::wcstod(value.c_str(), &end);
    while (end && *end == L' ') ++end;
    if (end == value.c_str() || (end && *end)) return nullptr;
    return IReference<double>(parsed);
  }
  IReference<int64_t> ParseInt(hstring const& text) const {
    auto value = ParseDouble(text);
    return value ? IReference<int64_t>(static_cast<int64_t>(std::llround(value.Value()))) : nullptr;
  }
  IReference<uint64_t> ParseUInt(hstring const& text) const {
    auto value = ParseDouble(text);
    return value && value.Value() >= 0 ? IReference<uint64_t>(static_cast<uint64_t>(std::llround(value.Value()))) : nullptr;
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
  com_ptr<NumberFormatter> formatter;
  /* List rows, without headers, and whether they alternate backgrounds. */
  std::vector<ListViewItem> rows;
  bool striped{};
  /* Navigation item context menus. */
  std::vector<std::shared_ptr<MenuTarget>> menus;

  ControlElement(int32_t kind, FrameworkElement const& value) : zpd_winui3_element(value), kind(kind) {}
  ~ControlElement() override;

  bool is_button() const {
    return kind == ZPD_WINUI3_CONTROL_BUTTON || kind == ZPD_WINUI3_CONTROL_TOGGLE_BUTTON ||
           kind == ZPD_WINUI3_CONTROL_CHECK_BOX || kind == ZPD_WINUI3_CONTROL_RADIO_BUTTON;
  }
};

/* Rich edit boxes by their ABI pointer, so menu roles can report their changes. */
std::unordered_map<void*, std::weak_ptr<ControlState>>& rich_boxes() {
  thread_local std::unordered_map<void*, std::weak_ptr<ControlState>> boxes;
  return boxes;
}

ControlElement::~ControlElement() {
  if (kind == ZPD_WINUI3_CONTROL_RICH_EDIT_BOX) rich_boxes().erase(get_abi(value));
}

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

/* Shows the declared text or the value formatted with the current digits. */
void refresh_number_box(ControlElement* element) {
  // Assigning the formatter again makes the box format its value anew.
  element->value.as<NumberBox>().NumberFormatter(*element->formatter);
}

void refresh_list_stripes(ControlElement* element) {
  auto stripe = element->striped ? theme_brush(L"SubtleFillColorSecondaryBrush") : nullptr;
  for (size_t index = 0; index < element->rows.size(); ++index) {
    if (stripe && index % 2 == 1) element->rows[index].Background(stripe);
    else element->rows[index].ClearValue(Control::BackgroundProperty());
  }
}

/* Selects the row with the state's id without reporting it. */
void select_list_row(ControlElement* element) {
  auto list = element->value.as<ListView>();
  auto const& id = element->state->list_id;
  for (auto const& row : element->rows) {
    if (!id.empty() && to_string(unbox_value_or<hstring>(row.Tag(), L"")) == id) {
      list.SelectedItem(row);
      return;
    }
  }
  list.SelectedItem(nullptr);
}

ListViewItem find_list_row(ControlElement* element, std::string const& id) {
  for (auto const& row : element->rows) {
    if (to_string(unbox_value_or<hstring>(row.Tag(), L"")) == id) return row;
  }
  return nullptr;
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

/* Finds the descendant with automation id `id` in our panel and content hierarchy. */
FrameworkElement find_identified(DependencyObject const& root, hstring const& id) {
  if (auto element = root.try_as<FrameworkElement>()) {
    if (Automation::AutomationProperties::GetAutomationId(element) == id) return element;
  }
  if (auto panel = root.try_as<Panel>()) {
    for (auto const& child : panel.Children()) {
      if (auto found = find_identified(child, id)) return found;
    }
  } else if (auto scroll = root.try_as<ScrollViewer>()) {
    if (auto content = scroll.Content().try_as<DependencyObject>()) return find_identified(content, id);
  } else if (auto content = root.try_as<ContentControl>()) {
    if (auto inner = content.Content().try_as<DependencyObject>()) return find_identified(inner, id);
  } else if (auto border = root.try_as<Border>()) {
    if (auto child = border.Child()) return find_identified(child, id);
  }
  return nullptr;
}

double or_zero(double value) { return std::isnan(value) ? 0.0 : value; }

/* The scroll offset along one axis that shows [start, start + length) as aligned. */
double reveal_offset(double start, double length, double offset, double viewport, int32_t alignment) {
  switch (alignment) {
    case ZPD_WINUI3_SCROLL_START: return start;
    case ZPD_WINUI3_SCROLL_CENTER: return start + length / 2 - viewport / 2;
    case ZPD_WINUI3_SCROLL_END: return start + length - viewport;
    default:
      if (start < offset) return start;
      if (start + length > offset + viewport) return std::min(start, start + length - viewport);
      return offset;
  }
}

FrameworkElement make_control(ControlElement*& element, int32_t kind) {
  switch (kind) {
    case ZPD_WINUI3_CONTROL_CANVAS: {
      Canvas canvas;
      canvas.Background(Media::SolidColorBrush(Windows::UI::Colors::Transparent()));
      element = new ControlElement(kind, canvas);
      canvas.SizeChanged([state = weak(element)](IInspectable const& sender, SizeChangedEventArgs const& args) {
        auto locked = state.lock();
        if (!locked) return;
        zpd_winui3_event event{};
        event.kind = ZPD_WINUI3_EVENT_SIZE_CHANGED;
        event.x = args.NewSize().Width;
        event.y = args.NewSize().Height;
        event.inset = title_bar_inset(sender.as<UIElement>());
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
      // Editing ends with Enter in a single-line box, or when focus moves away.
      box.PreviewKeyDown([state = weak(element)](IInspectable const& sender, Input::KeyRoutedEventArgs const& args) {
        auto box = sender.as<TextBox>();
        auto locked = state.lock();
        if (locked && args.Key() == Windows::System::VirtualKey::Enter && !box.AcceptsReturn()) locked->emit_commit(box.Text(), true);
      });
      box.LostFocus([state = weak(element)](IInspectable const& sender, RoutedEventArgs const&) {
        if (auto locked = state.lock()) locked->emit_commit(sender.as<TextBox>().Text(), false);
      });
      return box;
    }
    case ZPD_WINUI3_CONTROL_RICH_EDIT_BOX: {
      RichEditBox box;
      box.AcceptsReturn(true);
      box.TextWrapping(TextWrapping::Wrap);
      box.IsSpellCheckEnabled(true);
      ScrollViewer::SetVerticalScrollBarVisibility(box, ScrollBarVisibility::Auto);
      element = new ControlElement(kind, box);
      rich_boxes()[get_abi(box)] = element->state;
      box.TextChanged([state = weak(element)](IInspectable const& sender, RoutedEventArgs const&) {
        if (auto locked = state.lock()) locked->emit_rich(sender.as<RichEditBox>());
      });
      return box;
    }
    case ZPD_WINUI3_CONTROL_BUTTON: {
      Button button;
      element = new ControlElement(kind, button);
      button.Click([state = weak(element)](IInspectable const& sender, RoutedEventArgs const&) {
        auto locked = state.lock();
        if (!locked) return;
        // A menu button opens its context menu instead of reporting the click.
        auto button = sender.as<Button>();
        if (locked->shows_menu) {
          if (auto menu = button.ContextFlyout()) {
            Primitives::FlyoutShowOptions options;
            options.Placement(Primitives::FlyoutPlacementMode::BottomEdgeAlignedLeft);
            menu.ShowAt(button, options);
          }
          return;
        }
        zpd_winui3_event event{};
        event.kind = ZPD_WINUI3_EVENT_CLICKED;
        locked->emit(event);
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
        auto toggle = sender.as<Primitives::ToggleButton>();
        bool on = unbox_value_or<bool>(toggle.IsChecked(), false);
        // A kept state reports the click but shows the declared state until the view changes it.
        if (locked->keeps_state) toggle.IsChecked(box_value(locked->flag).as<IReference<bool>>());
        else locked->flag = on;
        zpd_winui3_event event{};
        event.kind = ZPD_WINUI3_EVENT_STATE_CHANGED;
        event.flag = on;
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
      element->formatter = make_self<NumberFormatter>(element->state);
      box.NumberFormatter(*element->formatter);
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
        locked->emit_string(ZPD_WINUI3_EVENT_NAVIGATION_SELECTED, id);
      });
      return navigation;
    }
    case ZPD_WINUI3_CONTROL_LIST_VIEW: {
      ListView list;
      list.SelectionMode(ListViewSelectionMode::Single);
      element = new ControlElement(kind, list);
      list.SelectionChanged([state = weak(element)](IInspectable const& sender, SelectionChangedEventArgs const&) {
        auto locked = state.lock();
        if (!locked) return;
        auto selected = sender.as<ListView>().SelectedItem();
        auto id = selected ? to_string(unbox_value_or<hstring>(selected.as<FrameworkElement>().Tag(), L"")) : std::string();
        if (id == locked->list_id) return;
        locked->list_id = id;
        locked->emit_string(ZPD_WINUI3_EVENT_LIST_SELECTION_CHANGED, id, !id.empty());
      });
      return list;
    }
    case ZPD_WINUI3_CONTROL_PROGRESS_RING: {
      ProgressRing ring;
      ring.IsActive(true);
      ring.IsIndeterminate(true);
      element = new ControlElement(kind, ring);
      return ring;
    }
    case ZPD_WINUI3_CONTROL_PROGRESS_BAR: {
      ProgressBar bar;
      bar.Minimum(0);
      bar.Maximum(1);
      bar.VerticalAlignment(VerticalAlignment::Center);
      element = new ControlElement(kind, bar);
      return bar;
    }
    case ZPD_WINUI3_CONTROL_SEPARATOR: {
      Grid root;
      Shapes::Rectangle line;
      auto brush = theme_brush(L"DividerStrokeColorDefaultBrush");
      line.Fill(brush ? brush : Media::SolidColorBrush(Windows::UI::Colors::Gray()));
      root.Children().Append(line);
      // The line runs along the longer side of the frame, centered across it.
      root.SizeChanged([line](IInspectable const&, SizeChangedEventArgs const& args) {
        bool across = args.NewSize().Width >= args.NewSize().Height;
        line.Width(across ? NAN : 1.0);
        line.Height(across ? 1.0 : NAN);
        line.HorizontalAlignment(across ? HorizontalAlignment::Stretch : HorizontalAlignment::Center);
        line.VerticalAlignment(across ? VerticalAlignment::Center : VerticalAlignment::Stretch);
      });
      element = new ControlElement(kind, root);
      return root;
    }
    default:
      throw hresult_invalid_argument(L"unknown control kind");
  }
}

Media::Brush text_color(zpd_winui3_text_style const& style) {
  switch (style.color) {
    case ZPD_WINUI3_TEXT_COLOR_SECONDARY: return theme_brush(L"TextFillColorSecondaryBrush");
    case ZPD_WINUI3_TEXT_COLOR_TERTIARY: return theme_brush(L"TextFillColorTertiaryBrush");
    case ZPD_WINUI3_TEXT_COLOR_ACCENT: return theme_brush(L"AccentTextFillColorPrimaryBrush");
    case ZPD_WINUI3_TEXT_COLOR_CUSTOM: return brush_from(style.custom);
    default: return nullptr;
  }
}

/* Applies the font, color, and alignment that TextBlock and TextBox share. */
template <typename Owner, typename T>
void apply_text_style(T const& text, zpd_winui3_text_style const& style) {
  auto const& font = style.font;
  if (font.size > 0) text.FontSize(font.size);
  else text.ClearValue(Owner::FontSizeProperty());
  if (font.weight > 0) text.FontWeight(Windows::UI::Text::FontWeight{static_cast<uint16_t>(font.weight)});
  else text.ClearValue(Owner::FontWeightProperty());
  text.FontStyle(font.italic ? Windows::UI::Text::FontStyle::Italic : Windows::UI::Text::FontStyle::Normal);
  if (font.family.length) text.FontFamily(Media::FontFamily(string_from(font.family)));
  else text.ClearValue(Owner::FontFamilyProperty());
  if (auto brush = text_color(style)) text.Foreground(brush);
  else text.ClearValue(Owner::ForegroundProperty());
  static constexpr TextAlignment alignments[] = {TextAlignment::Left, TextAlignment::Center, TextAlignment::Right};
  text.TextAlignment(alignments[std::clamp(style.alignment, 0, 2)]);
}

}  // namespace

namespace zpd {

void notify_rich_text_changed(RichEditBox const& box) {
  auto found = rich_boxes().find(get_abi(box));
  if (found == rich_boxes().end()) return;
  if (auto state = found->second.lock()) state->emit_rich(box);
}

}  // namespace zpd

extern "C" {

zpd_winui3_element* zpd_winui3_control_create(int32_t kind) {
  return create<zpd_winui3_element>([&] {
    ControlElement* element{};
    auto value = make_control(element, kind);
    // Frames come from the Rust layout, so templates must not impose minimums.
    // A progress bar's template draws its track at its minimum height, so that one stays.
    value.MinWidth(0);
    if (kind != ZPD_WINUI3_CONTROL_PROGRESS_BAR) value.MinHeight(0);
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
        case ZPD_WINUI3_CONTROL_RICH_EDIT_BOX: element->value.as<RichEditBox>().PlaceholderText(value); break;
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
        element->state->committed = value;
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
      case ZPD_WINUI3_CONTROL_NUMBER_BOX:
        // The box shows this text instead of its formatted value; empty text shows the value.
        if (element->state->display != std::wstring_view(value)) {
          element->state->display = value;
          refresh_number_box(element);
        }
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
          element->state->fraction_digits = static_cast<int32_t>(value);
          refresh_number_box(element);
        } else if (property == ZPD_WINUI3_NUMBER_VALUE) {
          bool changed = std::isnan(element->state->number) || element->state->number != value;
          element->state->number = value;
          box.Value(value);
          // The declared text may belong to the new value even when the box's value is unchanged.
          if (!changed) refresh_number_box(element);
        }
        break;
      }
      case ZPD_WINUI3_CONTROL_PROGRESS_BAR:
        if (property == ZPD_WINUI3_NUMBER_VALUE) element->value.as<ProgressBar>().Value(std::clamp(value, 0.0, 1.0));
        break;
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
        else if (element->is_button() && element->kind != ZPD_WINUI3_CONTROL_BUTTON) element->value.as<Primitives::ToggleButton>().IsChecked(box_value(value).as<IReference<bool>>());
        break;
      case ZPD_WINUI3_BOOL_ENABLED:
        if (auto control = element->value.try_as<Control>()) control.IsEnabled(value);
        else element->value.Opacity(value ? 1.0 : 0.4);
        break;
      case ZPD_WINUI3_BOOL_READ_ONLY:
        if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BOX) element->value.as<TextBox>().IsReadOnly(value);
        else if (element->kind == ZPD_WINUI3_CONTROL_RICH_EDIT_BOX) element->value.as<RichEditBox>().IsReadOnly(value);
        break;
      case ZPD_WINUI3_BOOL_MULTILINE:
        if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BOX) {
          auto box = element->value.as<TextBox>();
          box.AcceptsReturn(value);
          box.TextWrapping(value ? TextWrapping::Wrap : TextWrapping::NoWrap);
          ScrollViewer::SetVerticalScrollBarVisibility(box, value ? ScrollBarVisibility::Auto : ScrollBarVisibility::Hidden);
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
      case ZPD_WINUI3_BOOL_BORDERED:
        if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BOX) {
          auto box = element->value.as<TextBox>();
          if (value) box.ClearValue(Control::BorderThicknessProperty());
          else box.BorderThickness({0, 0, 0, 0});
        }
        break;
      case ZPD_WINUI3_BOOL_DRAWS_BACKGROUND:
        if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BOX) {
          auto box = element->value.as<TextBox>();
          if (value) box.ClearValue(Control::BackgroundProperty());
          else box.Background(Media::SolidColorBrush(Windows::UI::Colors::Transparent()));
        }
        break;
      case ZPD_WINUI3_BOOL_SELECTABLE:
        if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BLOCK) element->value.as<TextBlock>().IsTextSelectionEnabled(value);
        break;
      case ZPD_WINUI3_BOOL_STRIPED:
        if (element->kind == ZPD_WINUI3_CONTROL_LIST_VIEW) {
          element->striped = value;
          refresh_list_stripes(element);
        }
        break;
      case ZPD_WINUI3_BOOL_SHOWS_MENU: element->state->shows_menu = value; break;
      case ZPD_WINUI3_BOOL_KEEPS_STATE: element->state->keeps_state = value; break;
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

int32_t zpd_winui3_control_set_foreground(zpd_winui3_element* target, const zpd_winui3_color* value) {
  return status([&] {
    auto element = control(target);
    if (auto control = element->value.try_as<Control>()) {
      if (value) control.Foreground(brush_from(*value));
      else control.ClearValue(Control::ForegroundProperty());
    }
  });
}

int32_t zpd_winui3_control_get_color(const zpd_winui3_element* target, zpd_winui3_color* value) {
  return status([&] { *value = control(target)->state->color; });
}

int32_t zpd_winui3_control_set_text_style(zpd_winui3_element* target, const zpd_winui3_text_style* style) {
  return status([&] {
    auto element = control(target);
    if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BLOCK) {
      auto text = element->value.as<TextBlock>();
      apply_text_style<TextBlock>(text, *style);
      text.MaxLines(std::max(style->max_lines, 0));
      // A limit of one line truncates instead of wrapping.
      if (style->max_lines == 1) text.TextWrapping(TextWrapping::NoWrap);
      else if (style->max_lines > 1) text.TextWrapping(TextWrapping::WrapWholeWords);
    } else if (element->kind == ZPD_WINUI3_CONTROL_TEXT_BOX) {
      apply_text_style<Control>(element->value.as<TextBox>(), *style);
    }
  });
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

int32_t zpd_winui3_scroll_viewer_reveal(zpd_winui3_element* target, zpd_winui3_string id, int32_t alignment) {
  return status([&] {
    auto element = control(target);
    auto found = find_identified(element->document, string_from(id));
    if (!found) return;
    // Frames are set but may not be arranged yet, so the offsets are summed from Canvas positions.
    double x = 0, y = 0;
    for (FrameworkElement current = found; current && current != element->document; current = current.Parent().try_as<FrameworkElement>()) {
      x += or_zero(Canvas::GetLeft(current));
      y += or_zero(Canvas::GetTop(current));
    }
    auto scroll = element->value.as<ScrollViewer>();
    double viewport_width = scroll.ViewportWidth() > 0 ? scroll.ViewportWidth() : or_zero(scroll.Width());
    double viewport_height = scroll.ViewportHeight() > 0 ? scroll.ViewportHeight() : or_zero(scroll.Height());
    double max_x = std::max(0.0, or_zero(element->document.Width()) - viewport_width);
    double max_y = std::max(0.0, or_zero(element->document.Height()) - viewport_height);
    auto state = element->state;
    state->x = std::clamp(reveal_offset(x, or_zero(found.Width()), scroll.HorizontalOffset(), viewport_width, alignment), 0.0, max_x);
    state->y = std::clamp(reveal_offset(y, or_zero(found.Height()), scroll.VerticalOffset(), viewport_height, alignment), 0.0, max_y);
    state->scroll_pending = true;
    state->apply_pending_scroll(scroll);
  });
}

int32_t zpd_winui3_navigation_set_items(zpd_winui3_element* target, const zpd_winui3_navigation_item* items, size_t length) {
  return status([&] {
    auto element = control(target);
    auto navigation = element->value.as<NavigationView>();
    navigation.MenuItems().Clear();
    element->menus.clear();
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
      if (source.badge.length) {
        // Counts show as an info badge; other text trails the title.
        auto badge = std::string(bytes(source.badge));
        bool count = std::all_of(badge.begin(), badge.end(), [](char ch) { return ch >= '0' && ch <= '9'; }) && badge.size() < 10;
        if (count) {
          InfoBadge info;
          info.Value(std::stoi(badge));
          item.InfoBadge(info);
        } else {
          Grid content;
          ColumnDefinition title_column;
          title_column.Width(GridLengthHelper::FromValueAndType(1, GridUnitType::Star));
          content.ColumnDefinitions().Append(title_column);
          content.ColumnDefinitions().Append(ColumnDefinition());
          content.ColumnDefinitions().GetAt(1).Width(GridLengthHelper::Auto());
          TextBlock title;
          title.Text(string_from(source.title));
          title.TextTrimming(TextTrimming::CharacterEllipsis);
          TextBlock trailing;
          trailing.Text(string_from(source.badge));
          trailing.Margin({8, 0, 4, 0});
          if (auto brush = theme_brush(L"TextFillColorSecondaryBrush")) trailing.Foreground(brush);
          Grid::SetColumn(trailing, 1);
          content.Children().Append(title);
          content.Children().Append(trailing);
          item.Content(content);
        }
      }
      if (source.menu_length) {
        auto menu_target = std::make_shared<MenuTarget>();
        menu_target->handler = [state = weak(element)](std::string const& id) {
          if (auto locked = state.lock()) locked->emit_string(ZPD_WINUI3_EVENT_MENU_SELECTED, id);
        };
        MenuFlyout menu;
        append_menu_items(menu.Items(), menu_target, source.menu, source.menu_length);
        item.ContextFlyout(menu);
        element->menus.push_back(menu_target);
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

int32_t zpd_winui3_rich_text_set(zpd_winui3_element* target, const zpd_winui3_rich_text* text) {
  return status([&] {
    auto element = control(target);
    element->state->replacing = true;
    struct Reset {
      bool& value;
      ~Reset() { value = false; }
    } reset{element->state->replacing};
    set_rich_text(element->value.as<RichEditBox>(), *text);
  });
}

int32_t zpd_winui3_rich_text_get(const zpd_winui3_element* target, const void* data, zpd_winui3_rich_text_fn receive) {
  return status([&] {
    read_rich_text(control(target)->value.as<RichEditBox>(), [&](zpd_winui3_rich_text const& text) { receive(data, &text); });
  });
}

int32_t zpd_winui3_list_set_entries(zpd_winui3_element* target, const zpd_winui3_list_entry* entries, size_t length) {
  return status([&] {
    auto element = control(target);
    auto list = element->value.as<ListView>();
    // Cells move to new rows, so the old rows release them first. A removed row can outlive
    // this call (focus, pointer capture, automation), and `Parent()` does not report its panel.
    for (auto const& row : element->rows) {
      if (auto cells = row.Content().try_as<Panel>()) cells.Children().Clear();
      row.Content(nullptr);
    }
    element->rows.clear();
    list.Items().Clear();
    for (size_t index = 0; index < length; ++index) {
      auto const& entry = entries[index];
      ListViewItem item;
      item.MinHeight(0);
      item.HorizontalContentAlignment(HorizontalAlignment::Stretch);
      item.VerticalContentAlignment(VerticalAlignment::Stretch);
      if (entry.header) {
        // Headers cannot be clicked or reached with the keyboard.
        TextBlock title;
        title.Text(string_from(entry.title));
        title.FontWeight(Windows::UI::Text::FontWeights::SemiBold());
        title.Margin({0, 8, 0, 2});
        if (auto brush = theme_brush(L"TextFillColorSecondaryBrush")) title.Foreground(brush);
        item.Content(title);
        item.IsHitTestVisible(false);
        item.IsTabStop(false);
        list.Items().Append(item);
        continue;
      }
      Canvas row;
      for (size_t cell = 0; cell < entry.cells_length; ++cell) {
        detach(entry.cells[cell]->value);
        row.Children().Append(entry.cells[cell]->value);
      }
      item.Content(row);
      item.Tag(box_value(string_from(entry.id)));
      item.Margin({LIST_ROW_MARGIN, 1, LIST_ROW_MARGIN, 1});
      item.Padding({LIST_ROW_PADDING, 0, LIST_ROW_PADDING, 0});
      list.Items().Append(item);
      element->rows.push_back(item);
    }
    refresh_list_stripes(element);
    select_list_row(element);
  });
}

int32_t zpd_winui3_list_set_columns(zpd_winui3_element* target, const zpd_winui3_string* titles, const double* widths, size_t length) {
  return status([&] {
    auto list = control(target)->value.as<ListView>();
    if (!length) {
      list.Header(nullptr);
      return;
    }
    Grid header;
    header.Padding({LIST_ROW_MARGIN + LIST_ROW_PADDING, 4, LIST_ROW_MARGIN + LIST_ROW_PADDING, 4});
    header.BorderThickness({0, 0, 0, 1});
    if (auto brush = theme_brush(L"DividerStrokeColorDefaultBrush")) header.BorderBrush(brush);
    for (size_t index = 0; index < length; ++index) {
      ColumnDefinition column;
      column.Width(GridLengthHelper::FromPixels(std::max(widths[index], 0.0)));
      header.ColumnDefinitions().Append(column);
      TextBlock title;
      title.Text(string_from(titles[index]));
      title.FontWeight(Windows::UI::Text::FontWeights::SemiBold());
      title.TextTrimming(TextTrimming::CharacterEllipsis);
      Grid::SetColumn(title, static_cast<int32_t>(index));
      header.Children().Append(title);
    }
    list.Header(header);
  });
}

int32_t zpd_winui3_list_set_row_heights(zpd_winui3_element* target, const double* heights, size_t length) {
  return status([&] {
    auto element = control(target);
    for (size_t index = 0; index < length && index < element->rows.size(); ++index) {
      if (auto row = element->rows[index].Content().try_as<FrameworkElement>()) row.Height(std::max(heights[index], 1.0));
    }
  });
}

int32_t zpd_winui3_list_select(zpd_winui3_element* target, zpd_winui3_string id) {
  return status([&] {
    auto element = control(target);
    element->state->list_id = std::string(bytes(id));
    select_list_row(element);
  });
}

int32_t zpd_winui3_list_reveal(zpd_winui3_element* target, zpd_winui3_string id) {
  return status([&] {
    auto element = control(target);
    if (auto row = find_list_row(element, std::string(bytes(id)))) element->value.as<ListView>().ScrollIntoView(row);
  });
}

}  // extern "C"
