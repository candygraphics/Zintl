#include "common.h"

#include <algorithm>

#include <winrt/Microsoft.UI.Text.h>

using namespace zpd;
namespace Text = winrt::Microsoft::UI::Text;

namespace {

constexpr float BODY_SIZE = 14;
constexpr wchar_t MONOSPACE[] = L"Consolas";
constexpr int32_t DASH = 0x2013;
constexpr int32_t BOX = 0x2610;
constexpr int32_t CHECKED_BOX = 0x2611;

/* The base font and list marker of a paragraph style. */
struct Look {
  float size;
  int32_t weight;
  bool monospace;
  Text::MarkerType marker;
  int32_t start;
};

Look look(int32_t style) {
  switch (style) {
    case ZPD_WINUI3_PARAGRAPH_TITLE: return {28, 700, false, Text::MarkerType::None, 0};
    case ZPD_WINUI3_PARAGRAPH_HEADING: return {22, 700, false, Text::MarkerType::None, 0};
    case ZPD_WINUI3_PARAGRAPH_SUBHEADING: return {17, 600, false, Text::MarkerType::None, 0};
    case ZPD_WINUI3_PARAGRAPH_MONOSPACED: return {BODY_SIZE, 400, true, Text::MarkerType::None, 0};
    case ZPD_WINUI3_PARAGRAPH_BULLETED: return {BODY_SIZE, 400, false, Text::MarkerType::Bullet, 0};
    case ZPD_WINUI3_PARAGRAPH_DASHED: return {BODY_SIZE, 400, false, Text::MarkerType::UnicodeSequence, DASH};
    case ZPD_WINUI3_PARAGRAPH_NUMBERED: return {BODY_SIZE, 400, false, Text::MarkerType::Arabic, 1};
    case ZPD_WINUI3_PARAGRAPH_CHECKLIST: return {BODY_SIZE, 400, false, Text::MarkerType::UnicodeSequence, BOX};
    case ZPD_WINUI3_PARAGRAPH_CHECKLIST_DONE: return {BODY_SIZE, 400, false, Text::MarkerType::UnicodeSequence, CHECKED_BOX};
    default: return {BODY_SIZE, 400, false, Text::MarkerType::None, 0};
  }
}

/* Whether the style's base font is already bold, so a bold run adds nothing. */
bool is_bold(int32_t style) {
  return style == ZPD_WINUI3_PARAGRAPH_TITLE || style == ZPD_WINUI3_PARAGRAPH_HEADING || style == ZPD_WINUI3_PARAGRAPH_SUBHEADING;
}

hstring body_font(RichEditBox const& box) {
  auto family = box.FontFamily();
  return family ? family.Source() : hstring(L"Segoe UI");
}

/* Sets the paragraph's list marker and resets its characters to the style's base font. */
void apply_paragraph(Text::ITextRange const& range, int32_t style, hstring const& font) {
  auto base = look(style);
  auto paragraph = range.ParagraphFormat();
  paragraph.ListType(base.marker);
  if (base.marker == Text::MarkerType::UnicodeSequence || base.marker == Text::MarkerType::Arabic) paragraph.ListStart(base.start);
  if (base.marker == Text::MarkerType::Arabic) paragraph.ListStyle(Text::MarkerStyle::Period);
  if (base.marker != Text::MarkerType::None) paragraph.ListTab(18);
  range.ParagraphFormat(paragraph);
  auto characters = range.CharacterFormat();
  characters.Size(base.size);
  characters.Weight(base.weight);
  characters.Name(base.monospace ? hstring(MONOSPACE) : font);
  range.CharacterFormat(characters);
}

int32_t paragraph_style(Text::ITextRange const& range) {
  auto paragraph = range.ParagraphFormat();
  switch (paragraph.ListType()) {
    case Text::MarkerType::None:
    case Text::MarkerType::Undefined: break;
    case Text::MarkerType::Bullet: return ZPD_WINUI3_PARAGRAPH_BULLETED;
    case Text::MarkerType::UnicodeSequence:
      switch (paragraph.ListStart()) {
        case DASH: return ZPD_WINUI3_PARAGRAPH_DASHED;
        case BOX: return ZPD_WINUI3_PARAGRAPH_CHECKLIST;
        case CHECKED_BOX: return ZPD_WINUI3_PARAGRAPH_CHECKLIST_DONE;
        default: return ZPD_WINUI3_PARAGRAPH_BULLETED;
      }
    default: return ZPD_WINUI3_PARAGRAPH_NUMBERED;
  }
  auto characters = range.CharacterFormat();
  if (characters.Name() == MONOSPACE) return ZPD_WINUI3_PARAGRAPH_MONOSPACED;
  float size = characters.Size();
  if (size >= 26) return ZPD_WINUI3_PARAGRAPH_TITLE;
  if (size >= 20) return ZPD_WINUI3_PARAGRAPH_HEADING;
  if (size >= 16) return ZPD_WINUI3_PARAGRAPH_SUBHEADING;
  return ZPD_WINUI3_PARAGRAPH_BODY;
}

uint32_t traits(Text::ITextRange const& range, int32_t style) {
  auto characters = range.CharacterFormat();
  uint32_t result{};
  if (characters.Weight() >= 700 && !is_bold(style)) result |= ZPD_WINUI3_TRAIT_BOLD;
  if (characters.Italic() == Text::FormatEffect::On) result |= ZPD_WINUI3_TRAIT_ITALIC;
  if (characters.Underline() != Text::UnderlineType::None && characters.Underline() != Text::UnderlineType::Undefined) result |= ZPD_WINUI3_TRAIT_UNDERLINE;
  if (characters.Strikethrough() == Text::FormatEffect::On) result |= ZPD_WINUI3_TRAIT_STRIKETHROUGH;
  if (characters.Name() == MONOSPACE && style != ZPD_WINUI3_PARAGRAPH_MONOSPACED) result |= ZPD_WINUI3_TRAIT_MONOSPACE;
  return result;
}

/* The document text with '\r' paragraph marks, without the final mark RichEdit keeps. */
std::wstring document_text(Text::RichEditTextDocument const& document) {
  hstring text;
  document.GetText(Text::TextGetOptions::None, text);
  std::wstring result(text);
  if (!result.empty() && result.back() == L'\r') result.pop_back();
  return result;
}

int32_t current_style(RichEditBox const& box) {
  auto selection = box.Document().Selection();
  auto range = box.Document().GetRange(selection.StartPosition(), selection.StartPosition());
  range.Expand(Text::TextRangeUnit::Paragraph);
  return paragraph_style(range);
}

}  // namespace

namespace zpd {

void set_rich_text(RichEditBox const& box, zpd_winui3_rich_text const& text) {
  auto document = box.Document();
  auto font = body_font(box);
  std::wstring full;
  struct Span {
    int32_t start;
    int32_t end;
    uint32_t traits;
    hstring link;
  };
  std::vector<Span> spans;
  for (size_t index = 0; index < text.runs_length; ++index) {
    auto const& run = text.runs[index];
    std::wstring value(string_from(run.text));
    std::replace(value.begin(), value.end(), L'\n', L'\r');
    auto start = static_cast<int32_t>(full.size());
    full += value;
    if (run.traits || run.link.length) spans.push_back({start, static_cast<int32_t>(full.size()), run.traits, string_from(run.link)});
  }
  document.SetText(Text::TextSetOptions::None, full);
  auto all = document.GetRange(0, static_cast<int32_t>(full.size()) + 1);
  auto characters = all.CharacterFormat();
  characters.Italic(Text::FormatEffect::Off);
  characters.Underline(Text::UnderlineType::None);
  characters.Strikethrough(Text::FormatEffect::Off);
  all.CharacterFormat(characters);
  // Paragraphs end at each '\r'; the last one runs to the end.
  int32_t start = 0;
  for (size_t index = 0; index < std::max<size_t>(text.paragraphs_length, 1); ++index) {
    auto end = full.find(L'\r', static_cast<size_t>(start));
    int32_t stop = end == std::wstring::npos ? static_cast<int32_t>(full.size()) + 1 : static_cast<int32_t>(end) + 1;
    int32_t style = index < text.paragraphs_length ? text.paragraphs[index] : ZPD_WINUI3_PARAGRAPH_BODY;
    apply_paragraph(document.GetRange(start, stop), style, font);
    if (end == std::wstring::npos) break;
    start = stop;
  }
  for (auto const& span : spans) {
    auto range = document.GetRange(span.start, span.end);
    auto format = range.CharacterFormat();
    if (span.traits & ZPD_WINUI3_TRAIT_BOLD) format.Weight(700);
    if (span.traits & ZPD_WINUI3_TRAIT_ITALIC) format.Italic(Text::FormatEffect::On);
    if (span.traits & ZPD_WINUI3_TRAIT_UNDERLINE) format.Underline(Text::UnderlineType::Single);
    if (span.traits & ZPD_WINUI3_TRAIT_STRIKETHROUGH) format.Strikethrough(Text::FormatEffect::On);
    if (span.traits & ZPD_WINUI3_TRAIT_MONOSPACE) format.Name(MONOSPACE);
    range.CharacterFormat(format);
    if (!span.link.empty()) range.Link(L"\"" + span.link + L"\"");
  }
  document.Selection().SetRange(0, 0);
  document.ClearUndoRedoHistory();
}

void read_rich_text(RichEditBox const& box, std::function<void(zpd_winui3_rich_text const&)> const& body) {
  auto document = box.Document();
  auto text = document_text(document);
  auto length = static_cast<int32_t>(text.size());
  std::vector<int32_t> paragraphs;
  std::vector<int32_t> starts;
  for (int32_t start = 0;;) {
    starts.push_back(start);
    paragraphs.push_back(paragraph_style(document.GetRange(start, start)));
    auto end = text.find(L'\r', static_cast<size_t>(start));
    if (end == std::wstring::npos) break;
    start = static_cast<int32_t>(end) + 1;
  }
  // Runs split where formatting changes and where paragraphs end.
  std::vector<std::string> texts;
  std::vector<std::string> links;
  std::vector<uint32_t> run_traits;
  size_t paragraph = 0;
  for (int32_t position = 0; position < length;) {
    while (paragraph + 1 < starts.size() && starts[paragraph + 1] <= position) ++paragraph;
    auto range = document.GetRange(position, position + 1);
    range.Expand(Text::TextRangeUnit::CharacterFormat);
    int32_t end = std::min(range.EndPosition(), length);
    if (paragraph + 1 < starts.size()) end = std::min(end, starts[paragraph + 1]);
    end = std::max(end, position + 1);
    auto run = document.GetRange(position, end);
    std::wstring value = text.substr(static_cast<size_t>(position), static_cast<size_t>(end - position));
    std::replace(value.begin(), value.end(), L'\r', L'\n');
    std::wstring link(run.Link());
    if (link.size() >= 2 && link.front() == L'"' && link.back() == L'"') link = link.substr(1, link.size() - 2);
    texts.push_back(to_string(value));
    links.push_back(to_string(link));
    run_traits.push_back(traits(run, paragraphs[paragraph]));
    position = end;
  }
  std::vector<zpd_winui3_rich_run> runs;
  for (size_t index = 0; index < texts.size(); ++index) {
    auto const& value = texts[index];
    auto const& link = links[index];
    runs.push_back({{reinterpret_cast<uint8_t const*>(value.data()), value.size()}, run_traits[index], {reinterpret_cast<uint8_t const*>(link.data()), link.size()}});
  }
  body({runs.data(), runs.size(), paragraphs.data(), paragraphs.size()});
}

bool apply_rich_role(RichEditBox const& box, int32_t role) {
  auto document = box.Document();
  auto selection = document.Selection();
  auto characters = selection.CharacterFormat();
  switch (role) {
    case ZPD_WINUI3_MENU_ROLE_BOLD:
      characters.Bold(Text::FormatEffect::Toggle);
      selection.CharacterFormat(characters);
      break;
    case ZPD_WINUI3_MENU_ROLE_ITALIC:
      characters.Italic(Text::FormatEffect::Toggle);
      selection.CharacterFormat(characters);
      break;
    case ZPD_WINUI3_MENU_ROLE_UNDERLINE:
      characters.Underline(characters.Underline() == Text::UnderlineType::Single ? Text::UnderlineType::None : Text::UnderlineType::Single);
      selection.CharacterFormat(characters);
      break;
    default: {
      static constexpr int32_t styles[] = {ZPD_WINUI3_PARAGRAPH_TITLE, ZPD_WINUI3_PARAGRAPH_HEADING, ZPD_WINUI3_PARAGRAPH_SUBHEADING,
                                           ZPD_WINUI3_PARAGRAPH_BODY, ZPD_WINUI3_PARAGRAPH_MONOSPACED, ZPD_WINUI3_PARAGRAPH_BULLETED,
                                           ZPD_WINUI3_PARAGRAPH_DASHED, ZPD_WINUI3_PARAGRAPH_NUMBERED, ZPD_WINUI3_PARAGRAPH_CHECKLIST};
      if (role < ZPD_WINUI3_MENU_ROLE_TITLE || role > ZPD_WINUI3_MENU_ROLE_CHECKLIST) return false;
      int32_t style = styles[role - ZPD_WINUI3_MENU_ROLE_TITLE];
      // Choosing a list style again turns the paragraphs back into body text.
      int32_t current = current_style(box);
      bool list = style >= ZPD_WINUI3_PARAGRAPH_BULLETED;
      if (list && (current == style || (style == ZPD_WINUI3_PARAGRAPH_CHECKLIST && current == ZPD_WINUI3_PARAGRAPH_CHECKLIST_DONE))) style = ZPD_WINUI3_PARAGRAPH_BODY;
      auto range = document.GetRange(selection.StartPosition(), selection.EndPosition());
      range.Expand(Text::TextRangeUnit::Paragraph);
      apply_paragraph(range, style, body_font(box));
      break;
    }
  }
  // Formatting raises no TextChanged, so the change is reported here.
  notify_rich_text_changed(box);
  return true;
}

}  // namespace zpd
