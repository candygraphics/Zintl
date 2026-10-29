#include "common.h"

#include <vector>

#include <winrt/Microsoft.Windows.Storage.Pickers.h>

using namespace zpd;
using namespace winrt::Microsoft::UI::Dispatching;
using winrt::Windows::Foundation::AsyncStatus;
namespace Pickers = winrt::Microsoft::Windows::Storage::Pickers;

struct zpd_winui3_popover {
  Flyout flyout;
  Callback callback;
  zpd_winui3_invoke_fn closed{};
  event_token opened_token{};
  event_token closed_token{};
  bool open{};
};

namespace {

/* A dialog or picker that reports its result once on the UI thread. */
struct SheetState {
  Callback callback;
  DispatcherQueue queue{DispatcherQueue::GetForCurrentThread()};
  std::shared_ptr<int> sheets;
  bool finished{};

  explicit SheetState(std::shared_ptr<int> sheets, Callback callback)
      : callback(std::move(callback)), sheets(std::move(sheets)) {
    ++*this->sheets;
  }

  /* Settles the count when the operation failed before completing. */
  ~SheetState() {
    if (!finished) --*sheets;
  }

  /* Runs `report` once with the Rust callback data, unless it was released. */
  template <typename F>
  static void finish(std::shared_ptr<SheetState> const& state, F report) {
    state->queue.TryEnqueue([state, report = std::move(report)] {
      if (state->finished) return;
      state->finished = true;
      --*state->sheets;
      if (state->callback.data) report(state->callback.data);
    });
  }
};

Primitives::FlyoutPlacementMode placement_from(int32_t placement) {
  switch (placement) {
    case ZPD_WINUI3_PLACEMENT_TOP: return Primitives::FlyoutPlacementMode::Top;
    case ZPD_WINUI3_PLACEMENT_LEFT: return Primitives::FlyoutPlacementMode::Left;
    case ZPD_WINUI3_PLACEMENT_RIGHT: return Primitives::FlyoutPlacementMode::Right;
    default: return Primitives::FlyoutPlacementMode::Bottom;
  }
}

void report_paths(std::shared_ptr<SheetState> const& state, zpd_winui3_paths_fn completed, bool picked, std::vector<std::string> paths) {
  SheetState::finish(state, [completed, picked, paths = std::move(paths)](const void* data) {
    std::vector<zpd_winui3_string> values;
    for (auto const& path : paths) values.push_back({reinterpret_cast<uint8_t const*>(path.data()), path.size()});
    completed(data, picked && !paths.empty(), values.data(), values.size());
  });
}

}  // namespace

struct zpd_winui3_dialog {
  ContentDialog dialog{nullptr};
  std::shared_ptr<SheetState> state;
};

extern "C" {

zpd_winui3_popover* zpd_winui3_popover_create(const zpd_winui3_element* content, const void* data, zpd_winui3_invoke_fn closed, zpd_winui3_release_fn release) {
  Callback callback(data, release);
  return create<zpd_winui3_popover>([&] {
    auto popover = new zpd_winui3_popover();
    popover->callback = std::move(callback);
    popover->closed = closed;
    popover->flyout.Content(content->value);
    popover->opened_token = popover->flyout.Opened([popover](auto&&, auto&&) { popover->open = true; });
    popover->closed_token = popover->flyout.Closed([popover](auto&&, auto&&) {
      popover->open = false;
      if (popover->closed && popover->callback.data) popover->closed(popover->callback.data);
    });
    return popover;
  });
}

void zpd_winui3_popover_release(zpd_winui3_popover* popover) {
  if (!popover) return;
  try {
    popover->flyout.Opened(popover->opened_token);
    popover->flyout.Closed(popover->closed_token);
    if (popover->open) popover->flyout.Hide();
    popover->flyout.Content(nullptr);
  } catch (...) {
  }
  delete popover;
}

int32_t zpd_winui3_popover_show(zpd_winui3_popover* popover, const zpd_winui3_element* anchor, int32_t placement) {
  return status([&] {
    popover->flyout.Placement(placement_from(placement));
    popover->flyout.ShowAt(anchor->value);
    popover->open = true;
  });
}

int32_t zpd_winui3_popover_hide(zpd_winui3_popover* popover) {
  return status([&] { popover->flyout.Hide(); });
}

bool zpd_winui3_popover_is_open(const zpd_winui3_popover* popover) { return popover && popover->open; }

zpd_winui3_dialog* zpd_winui3_dialog_show(zpd_winui3_window* window, zpd_winui3_string title, zpd_winui3_string message, zpd_winui3_string primary, zpd_winui3_string secondary, zpd_winui3_string close, int32_t default_button, const void* data, zpd_winui3_int_fn completed, zpd_winui3_release_fn release) {
  Callback callback(data, release);
  return create<zpd_winui3_dialog>([&] {
    ContentDialog dialog;
    dialog.XamlRoot(window->xaml_root());
    dialog.Title(box_value(string_from(title)));
    if (message.length) {
      TextBlock text;
      text.Text(string_from(message));
      text.TextWrapping(TextWrapping::Wrap);
      dialog.Content(text);
    }
    dialog.PrimaryButtonText(string_from(primary));
    dialog.SecondaryButtonText(string_from(secondary));
    dialog.CloseButtonText(string_from(close));
    switch (default_button) {
      case ZPD_WINUI3_DIALOG_PRIMARY: dialog.DefaultButton(ContentDialogButton::Primary); break;
      case ZPD_WINUI3_DIALOG_SECONDARY: dialog.DefaultButton(ContentDialogButton::Secondary); break;
      case ZPD_WINUI3_DIALOG_CLOSE: dialog.DefaultButton(ContentDialogButton::Close); break;
      default: break;
    }
    auto state = std::make_shared<SheetState>(window->sheets, std::move(callback));
    // The close button also reports ContentDialogResult::None, so record it here.
    auto closed_by_button = std::make_shared<bool>(false);
    dialog.CloseButtonClick([closed_by_button](auto&&, auto&&) { *closed_by_button = true; });
    dialog.ShowAsync().Completed([state, completed, closed_by_button](auto const& operation, AsyncStatus status) {
      int32_t result = ZPD_WINUI3_DIALOG_NONE;
      if (status == AsyncStatus::Completed) {
        auto value = operation.GetResults();
        if (value == ContentDialogResult::Primary) result = ZPD_WINUI3_DIALOG_PRIMARY;
        else if (value == ContentDialogResult::Secondary) result = ZPD_WINUI3_DIALOG_SECONDARY;
        else if (*closed_by_button) result = ZPD_WINUI3_DIALOG_CLOSE;
      }
      SheetState::finish(state, [completed, result](const void* data) { completed(data, result); });
    });
    return new zpd_winui3_dialog{dialog, state};
  });
}

void zpd_winui3_dialog_release(zpd_winui3_dialog* dialog) {
  if (!dialog) return;
  try {
    // A released dialog reports nothing; hiding still settles its sheet count.
    dialog->state->callback.reset();
    dialog->dialog.Hide();
  } catch (...) {
  }
  delete dialog;
}

int32_t zpd_winui3_file_dialog_show(zpd_winui3_window* window, const zpd_winui3_file_dialog_options* options, const void* data, zpd_winui3_paths_fn completed, zpd_winui3_release_fn release) {
  Callback callback(data, release);
  return status([&] {
    auto id = window->window.AppWindow().Id();
    std::vector<hstring> extensions;
    for (size_t index = 0; index < options->extensions_length; ++index) {
      extensions.push_back(L"." + string_from(options->extensions[index]));
    }
    auto commit = string_from(options->commit_text);
    auto state = std::make_shared<SheetState>(window->sheets, std::move(callback));
    switch (options->mode) {
      case ZPD_WINUI3_FILE_DIALOG_SAVE: {
        Pickers::FileSavePicker picker(id);
        if (options->file_name.length) picker.SuggestedFileName(string_from(options->file_name));
        if (!commit.empty()) picker.CommitButtonText(commit);
        if (!extensions.empty()) picker.FileTypeChoices().Insert(L"Files", single_threaded_vector(std::vector<hstring>(extensions)));
        picker.PickSaveFileAsync().Completed([state, completed](auto const& operation, AsyncStatus status) {
          std::vector<std::string> paths;
          if (status == AsyncStatus::Completed) {
            if (auto result = operation.GetResults()) paths.push_back(to_string(result.Path()));
          }
          report_paths(state, completed, true, std::move(paths));
        });
        break;
      }
      case ZPD_WINUI3_FILE_DIALOG_FOLDER: {
        Pickers::FolderPicker picker(id);
        if (!commit.empty()) picker.CommitButtonText(commit);
        picker.PickSingleFolderAsync().Completed([state, completed](auto const& operation, AsyncStatus status) {
          std::vector<std::string> paths;
          if (status == AsyncStatus::Completed) {
            if (auto result = operation.GetResults()) paths.push_back(to_string(result.Path()));
          }
          report_paths(state, completed, true, std::move(paths));
        });
        break;
      }
      default: {
        Pickers::FileOpenPicker picker(id);
        if (!commit.empty()) picker.CommitButtonText(commit);
        if (extensions.empty()) picker.FileTypeFilter().Append(L"*");
        for (auto const& extension : extensions) picker.FileTypeFilter().Append(extension);
        if (options->multiple) {
          picker.PickMultipleFilesAsync().Completed([state, completed](auto const& operation, AsyncStatus status) {
            std::vector<std::string> paths;
            if (status == AsyncStatus::Completed) {
              for (auto const& result : operation.GetResults()) paths.push_back(to_string(result.Path()));
            }
            report_paths(state, completed, true, std::move(paths));
          });
        } else {
          picker.PickSingleFileAsync().Completed([state, completed](auto const& operation, AsyncStatus status) {
            std::vector<std::string> paths;
            if (status == AsyncStatus::Completed) {
              if (auto result = operation.GetResults()) paths.push_back(to_string(result.Path()));
            }
            report_paths(state, completed, true, std::move(paths));
          });
        }
        break;
      }
    }
  });
}

}  // extern "C"
