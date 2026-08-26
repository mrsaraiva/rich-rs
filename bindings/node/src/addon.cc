#include "common.h"

#include <array>
#include <cstdint>
#include <memory>
#include <string>
#include <string_view>
#include <thread>
#include <utility>
#include <vector>

namespace rich_node {
namespace {

using Callback = Napi::Value (*)(const Napi::CallbackInfo&);

#define CLOSE_CALLBACK(callback_name, kind_value, operation)                                       \
  Napi::Value callback_name(const Napi::CallbackInfo& info) {                                      \
    Owner* owner = ThisOwner(info, kind_value, operation, false);                                  \
    CloseOwner(owner);                                                                             \
    return info.Env().Undefined();                                                                 \
  }

CLOSE_CALLBACK(ConsoleClose, OwnerKind::Console, "Console.close")
CLOSE_CALLBACK(RenderableClose, OwnerKind::Renderable, "Renderable.close")
CLOSE_CALLBACK(TextClose, OwnerKind::Text, "Text.close")
CLOSE_CALLBACK(StyleClose, OwnerKind::Style, "Style.close")
CLOSE_CALLBACK(TableClose, OwnerKind::Table, "Table.close")
CLOSE_CALLBACK(PanelClose, OwnerKind::Panel, "Panel.close")
CLOSE_CALLBACK(TreeClose, OwnerKind::Tree, "Tree.close")
CLOSE_CALLBACK(RuleClose, OwnerKind::Rule, "Rule.close")
CLOSE_CALLBACK(ColumnsClose, OwnerKind::Columns, "Columns.close")
CLOSE_CALLBACK(AlignClose, OwnerKind::Align, "Align.close")
CLOSE_CALLBACK(PaddingClose, OwnerKind::Padding, "Padding.close")
CLOSE_CALLBACK(SyntaxClose, OwnerKind::Syntax, "Syntax.close")
CLOSE_CALLBACK(PrettyClose, OwnerKind::Pretty, "Pretty.close")
CLOSE_CALLBACK(MarkdownClose, OwnerKind::Markdown, "Markdown.close")
CLOSE_CALLBACK(JsonClose, OwnerKind::Json, "Json.close")
CLOSE_CALLBACK(ProgressClose, OwnerKind::Progress, "Progress.close")
CLOSE_CALLBACK(StatusClose, OwnerKind::Status, "Status.close")
CLOSE_CALLBACK(SpinnerClose, OwnerKind::Spinner, "Spinner.close")

Callback CloseForKind(OwnerKind kind) {
  switch (kind) {
  case OwnerKind::Console:
    return ConsoleClose;
  case OwnerKind::Renderable:
    return RenderableClose;
  case OwnerKind::Text:
    return TextClose;
  case OwnerKind::Style:
    return StyleClose;
  case OwnerKind::Table:
    return TableClose;
  case OwnerKind::Panel:
    return PanelClose;
  case OwnerKind::Tree:
    return TreeClose;
  case OwnerKind::Rule:
    return RuleClose;
  case OwnerKind::Columns:
    return ColumnsClose;
  case OwnerKind::Align:
    return AlignClose;
  case OwnerKind::Padding:
    return PaddingClose;
  case OwnerKind::Syntax:
    return SyntaxClose;
  case OwnerKind::Pretty:
    return PrettyClose;
  case OwnerKind::Markdown:
    return MarkdownClose;
  case OwnerKind::Json:
    return JsonClose;
  case OwnerKind::Progress:
    return ProgressClose;
  case OwnerKind::Status:
    return StatusClose;
  case OwnerKind::Spinner:
    return SpinnerClose;
  case OwnerKind::Count:
    break;
  }
  return nullptr;
}

void InitializeEmptyOwner(const Napi::CallbackInfo& info, OwnerKind kind) {
  auto* owner = new Owner{.kind = kind, .pointer = nullptr, .free_pointer = FreeFunction(kind)};
  WrapOwner(info.Env(), info.This().As<Napi::Object>(), owner);
}

bool BeginConstructor(const Napi::CallbackInfo& info, OwnerKind kind) {
  EnsureConstructCall(info, KindName(kind));
  if (IsInternalConstruction(info)) {
    InitializeEmptyOwner(info, kind);
    return true;
  }
  EnsureAbi(info.Env());
  return false;
}

constexpr napi_type_tag kRenderOptionsTag{UINT64_C(0x726963682d6f7074),
                                          UINT64_C(0x696f6e7300000002)};
constexpr napi_type_tag kMeasurementTag{UINT64_C(0x726963682d6d6561), UINT64_C(0x7375726500000002)};
constexpr napi_type_tag kSegmentSinkTag{UINT64_C(0x726963682d73696e), UINT64_C(0x6b00000000000002)};

struct RenderOptionsValue {
  RichRenderOptions value;
};

struct MeasurementValue {
  RichMeasurement value;
};

struct SegmentSinkValue {
  RichSegmentSink* sink;
  std::thread::id thread;
  bool active;
};

struct CustomContext {
  napi_env env;
  AddonData* data;
  std::thread::id thread;
  napi_ref renderer;
  napi_ref render;
  napi_ref measure;
};

bool ContextMatchesEnvironment(const CustomContext* context) noexcept {
  if (context == nullptr || context->thread != std::this_thread::get_id() ||
      context->data->creator_thread != std::this_thread::get_id()) {
    return false;
  }
  void* instance = nullptr;
  return napi_get_instance_data(context->env, &instance) == napi_ok && instance == context->data;
}

thread_local RenderOptionsValue* g_pending_render_options = nullptr;
thread_local SegmentSinkValue* g_pending_segment_sink = nullptr;

template <typename Value> void FinalizeValue(napi_env, void* raw, void*) {
  delete static_cast<Value*>(raw);
}

void DefineReadonly(Napi::Object object, const char* name, Napi::Value value) {
  object.DefineProperty(Napi::PropertyDescriptor::Value(
      name, value, static_cast<napi_property_attributes>(napi_enumerable)));
}

void FreezeObject(Napi::Env env, Napi::Object object) {
  if (napi_object_freeze(env, object) != napi_ok) {
    throw Napi::Error::New(env, "failed to freeze immutable binding value");
  }
}

template <typename Value>
Value* UnwrapValue(Napi::Env env, const Napi::Value& input, const Napi::FunctionReference& ctor,
                   const napi_type_tag* tag, std::string_view operation,
                   std::string_view expected) {
  if (!input.IsObject()) {
    ThrowType(env, std::string(operation) + ": expected " + std::string(expected));
  }
  Napi::Object object = input.As<Napi::Object>();
  if (!object.InstanceOf(ctor.Value())) {
    ThrowType(env, std::string(operation) + ": expected " + std::string(expected));
  }
  bool tagged = false;
  void* raw = nullptr;
  if (napi_check_object_type_tag(env, object, tag, &tagged) != napi_ok || !tagged ||
      napi_unwrap(env, object, &raw) != napi_ok || raw == nullptr) {
    ThrowType(env, std::string(operation) + ": expected " + std::string(expected));
  }
  return static_cast<Value*>(raw);
}

Napi::Value RenderOptionsConstructor(const Napi::CallbackInfo& info) {
  EnsureConstructCall(info, "RenderOptions");
  AddonData* data = Data(info.Env());
  if (info.Length() != 1 || !data->secret.Value().StrictEquals(info[0]) ||
      g_pending_render_options == nullptr) {
    ThrowType(info.Env(), "RenderOptions cannot be constructed directly");
  }
  auto* value = g_pending_render_options;
  Napi::Object object = info.This().As<Napi::Object>();
  if (napi_type_tag_object(info.Env(), object, &kRenderOptionsTag) != napi_ok ||
      napi_wrap(info.Env(), object, value, FinalizeValue<RenderOptionsValue>, nullptr, nullptr) !=
          napi_ok) {
    g_pending_render_options = nullptr;
    delete value;
    throw Napi::Error::New(info.Env(), "failed to create RenderOptions");
  }
  g_pending_render_options = nullptr;
  DefineReadonly(object, "minWidth", Napi::Number::New(info.Env(), value->value.min_width));
  DefineReadonly(object, "maxWidth", Napi::Number::New(info.Env(), value->value.max_width));
  DefineReadonly(object, "maxHeight", Napi::Number::New(info.Env(), value->value.max_height));
  DefineReadonly(object, "colorSystem", Napi::Number::New(info.Env(), value->value.color_system));
  DefineReadonly(object, "isTerminal",
                 Napi::Boolean::New(info.Env(), value->value.is_terminal != 0));
  FreezeObject(info.Env(), object);
  return object;
}

Napi::Value MeasurementConstructor(const Napi::CallbackInfo& info) {
  EnsureConstructCall(info, "Measurement");
  if (info.Length() != 2) {
    ThrowType(info.Env(), "Measurement: expected minimum and maximum");
  }
  auto* value = new MeasurementValue{
      .value = RichMeasurement{.minimum = Uint32Arg(info[0], "Measurement minimum"),
                               .maximum = Uint32Arg(info[1], "Measurement maximum")}};
  Napi::Object object = info.This().As<Napi::Object>();
  if (napi_type_tag_object(info.Env(), object, &kMeasurementTag) != napi_ok ||
      napi_wrap(info.Env(), object, value, FinalizeValue<MeasurementValue>, nullptr, nullptr) !=
          napi_ok) {
    delete value;
    throw Napi::Error::New(info.Env(), "failed to create Measurement");
  }
  DefineReadonly(object, "minimum", Napi::Number::New(info.Env(), value->value.minimum));
  DefineReadonly(object, "maximum", Napi::Number::New(info.Env(), value->value.maximum));
  FreezeObject(info.Env(), object);
  return object;
}

Napi::Value SegmentSinkConstructor(const Napi::CallbackInfo& info) {
  EnsureConstructCall(info, "SegmentSink");
  AddonData* data = Data(info.Env());
  if (info.Length() != 1 || !data->secret.Value().StrictEquals(info[0]) ||
      g_pending_segment_sink == nullptr) {
    ThrowType(info.Env(), "SegmentSink cannot be constructed directly");
  }
  auto* value = g_pending_segment_sink;
  Napi::Object object = info.This().As<Napi::Object>();
  if (napi_type_tag_object(info.Env(), object, &kSegmentSinkTag) != napi_ok ||
      napi_wrap(info.Env(), object, value, FinalizeValue<SegmentSinkValue>, nullptr, nullptr) !=
          napi_ok) {
    g_pending_segment_sink = nullptr;
    delete value;
    throw Napi::Error::New(info.Env(), "failed to create SegmentSink");
  }
  g_pending_segment_sink = nullptr;
  return object;
}

SegmentSinkValue* ThisSink(const Napi::CallbackInfo& info, std::string_view operation) {
  AddonData* data = Data(info.Env());
  SegmentSinkValue* value =
      UnwrapValue<SegmentSinkValue>(info.Env(), info.This(), data->segment_sink_constructor,
                                    &kSegmentSinkTag, operation, "SegmentSink");
  if (!InCustomCallback() || !value->active || value->sink == nullptr ||
      value->thread != std::this_thread::get_id()) {
    ThrowRich(info.Env(), RICH_ERROR_INVALID_STATE, operation,
              "SegmentSink is outside its callback scope");
  }
  return value;
}

Napi::Value SegmentSinkPush(const Napi::CallbackInfo& info) {
  SegmentSinkValue* value = ThisSink(info, "SegmentSink.push");
  Utf8Arg text(info[0], "SegmentSink.push text");
  RichStr style{nullptr, 0};
  std::unique_ptr<Utf8Arg> style_storage;
  if (info.Length() > 1 && !info[1].IsUndefined()) {
    style_storage = std::make_unique<Utf8Arg>(info[1], "SegmentSink.push style");
    style = style_storage->native();
  }
  CheckResult(info.Env(), rich_segment_sink_push(value->sink, text.native(), style),
              "SegmentSink.push");
  return info.This();
}

Napi::Value SegmentSinkLine(const Napi::CallbackInfo& info) {
  SegmentSinkValue* value = ThisSink(info, "SegmentSink.line");
  CheckResult(info.Env(), rich_segment_sink_line(value->sink), "SegmentSink.line");
  return info.This();
}

Napi::Object MakeRenderOptions(CustomContext* context, const RichRenderOptions* options) {
  if (g_pending_render_options != nullptr) {
    throw Napi::Error::New(context->env, "nested RenderOptions construction");
  }
  g_pending_render_options = new RenderOptionsValue{.value = *options};
  try {
    return context->data->render_options_constructor.New({context->data->secret.Value()});
  } catch (...) {
    delete g_pending_render_options;
    g_pending_render_options = nullptr;
    throw;
  }
}

Napi::Object MakeSegmentSink(CustomContext* context, RichSegmentSink* sink,
                             SegmentSinkValue** out_value) {
  if (g_pending_segment_sink != nullptr) {
    throw Napi::Error::New(context->env, "nested SegmentSink construction");
  }
  g_pending_segment_sink =
      new SegmentSinkValue{.sink = sink, .thread = context->thread, .active = true};
  *out_value = g_pending_segment_sink;
  try {
    return context->data->segment_sink_constructor.New({context->data->secret.Value()});
  } catch (...) {
    *out_value = nullptr;
    delete g_pending_segment_sink;
    g_pending_segment_sink = nullptr;
    throw;
  }
}

void CaptureCallbackFailure(CustomContext* context, const char* fallback) noexcept {
  napi_value exception = nullptr;
  if (napi_get_and_clear_last_exception(context->env, &exception) == napi_ok &&
      exception != nullptr) {
    RecordPendingException(context->env, exception);
  } else {
    RecordPendingError(context->env, fallback);
  }
}

RichResult CustomRender(void* raw, const RichRenderOptions* options,
                        RichSegmentSink* sink) noexcept {
  auto* context = static_cast<CustomContext*>(raw);
  if (context == nullptr || options == nullptr || sink == nullptr ||
      !ContextMatchesEnvironment(context) || InCustomCallback()) {
    return RICH_ERROR_INVALID_STATE;
  }
  SegmentSinkValue* sink_value = nullptr;
  EnterCustomCallback();
  try {
    Napi::Object options_object = MakeRenderOptions(context, options);
    Napi::Object sink_object = MakeSegmentSink(context, sink, &sink_value);
    napi_value renderer = nullptr;
    napi_value render = nullptr;
    napi_value arguments[]{options_object, sink_object};
    napi_value result = nullptr;
    napi_status status = napi_get_reference_value(context->env, context->renderer, &renderer);
    if (status == napi_ok)
      status = napi_get_reference_value(context->env, context->render, &render);
    if (status == napi_ok) {
      status = napi_call_function(context->env, renderer, render, 2, arguments, &result);
    }
    napi_valuetype result_type = napi_undefined;
    if (status == napi_ok)
      status = napi_typeof(context->env, result, &result_type);
    sink_value->active = false;
    sink_value->sink = nullptr;
    LeaveCustomCallback();
    if (status != napi_ok) {
      CaptureCallbackFailure(context, "custom render callback failed");
      return RICH_ERROR_RENDER;
    }
    if (result_type != napi_undefined)
      return RICH_ERROR_RENDER;
    ++context->data->test_custom_renders;
    return RICH_OK;
  } catch (const Napi::Error& error) {
    if (sink_value != nullptr) {
      sink_value->active = false;
      sink_value->sink = nullptr;
    }
    LeaveCustomCallback();
    RecordPendingException(context->env, error.Value());
    return RICH_ERROR_RENDER;
  } catch (const std::exception& error) {
    if (sink_value != nullptr) {
      sink_value->active = false;
      sink_value->sink = nullptr;
    }
    LeaveCustomCallback();
    RecordPendingError(context->env, error.what());
    return RICH_ERROR_RENDER;
  } catch (...) {
    if (sink_value != nullptr) {
      sink_value->active = false;
      sink_value->sink = nullptr;
    }
    LeaveCustomCallback();
    RecordPendingError(context->env, "custom render callback threw an unknown exception");
    return RICH_ERROR_RENDER;
  }
}

RichResult CustomMeasure(void* raw, const RichRenderOptions* options,
                         RichMeasurement* output) noexcept {
  auto* context = static_cast<CustomContext*>(raw);
  if (context == nullptr || options == nullptr || output == nullptr ||
      context->measure == nullptr || !ContextMatchesEnvironment(context) || InCustomCallback()) {
    return RICH_ERROR_INVALID_STATE;
  }
  EnterCustomCallback();
  try {
    Napi::Object options_object = MakeRenderOptions(context, options);
    napi_value renderer = nullptr;
    napi_value measure = nullptr;
    napi_value argument = options_object;
    napi_value result = nullptr;
    napi_status status = napi_get_reference_value(context->env, context->renderer, &renderer);
    if (status == napi_ok)
      status = napi_get_reference_value(context->env, context->measure, &measure);
    if (status == napi_ok) {
      status = napi_call_function(context->env, renderer, measure, 1, &argument, &result);
    }
    if (status != napi_ok) {
      LeaveCustomCallback();
      CaptureCallbackFailure(context, "custom measure callback failed");
      return RICH_ERROR_RENDER;
    }
    MeasurementValue* measurement = UnwrapValue<MeasurementValue>(
        context->env, Napi::Value(context->env, result), context->data->measurement_constructor,
        &kMeasurementTag, "customRenderable.measure", "Measurement");
    *output = measurement->value;
    LeaveCustomCallback();
    return RICH_OK;
  } catch (const Napi::Error& error) {
    LeaveCustomCallback();
    RecordPendingException(context->env, error.Value());
    return RICH_ERROR_RENDER;
  } catch (const std::exception& error) {
    LeaveCustomCallback();
    RecordPendingError(context->env, error.what());
    return RICH_ERROR_RENDER;
  } catch (...) {
    LeaveCustomCallback();
    RecordPendingError(context->env, "custom measure callback threw an unknown exception");
    return RICH_ERROR_RENDER;
  }
}

void DestroyCustomContext(CustomContext* context, bool native_drop) noexcept {
  if (context == nullptr)
    return;
  if (ContextMatchesEnvironment(context)) {
    if (context->measure != nullptr)
      napi_delete_reference(context->env, context->measure);
    if (context->render != nullptr)
      napi_delete_reference(context->env, context->render);
    if (context->renderer != nullptr)
      napi_delete_reference(context->env, context->renderer);
    if (native_drop)
      ++context->data->test_custom_drops;
  }
  delete context;
}

void CustomDrop(void* raw) noexcept {
  DestroyCustomContext(static_cast<CustomContext*>(raw), true);
}

void SetPrototypeMethod(Napi::Env env, Napi::Function constructor, const char* name,
                        Callback callback) {
  constructor.Get("prototype")
      .As<Napi::Object>()
      .Set(name, Napi::Function::New(env, callback, name));
}

void SetStaticMethod(Napi::Env env, Napi::Function constructor, const char* name,
                     Callback callback) {
  constructor.Set(name, Napi::Function::New(env, callback, name));
}

Napi::Function RegisterClass(Napi::Env env, Napi::Object exports, OwnerKind kind,
                             Callback constructor_callback,
                             std::initializer_list<std::pair<const char*, Callback>> methods) {
  const char* name = KindName(kind);
  Napi::Function constructor = Napi::Function::New(env, constructor_callback, name);
  SetPrototypeMethod(env, constructor, "close", CloseForKind(kind));
  for (const auto& [method_name, callback] : methods) {
    SetPrototypeMethod(env, constructor, method_name, callback);
  }
  Data(env)->constructors.at(KindIndex(kind)) = Napi::Persistent(constructor);
  exports.Set(name, constructor);
  return constructor;
}

Napi::Value Configure(const Napi::CallbackInfo& info) {
  if (info.Length() != 1 || !info[0].IsFunction()) {
    ThrowType(info.Env(), "_configure: expected the RichError constructor");
  }
  AddonData* data = Data(info.Env());
  data->rich_error.Reset();
  data->rich_error = Napi::Persistent(info[0].As<Napi::Function>());
  return info.Env().Undefined();
}

Napi::Value EnsureAbiCallback(const Napi::CallbackInfo& info) {
  EnsureAbi(info.Env());
  return info.Env().Undefined();
}

Napi::Value VersionCallback(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  EnsureAbi(env);
  BufferGuard buffer;
  CheckResult(env, rich_ffi_version(buffer.output()), "version");
  return BufferToString(env, buffer, "version");
}

Napi::Value RenderableConstructor(const Napi::CallbackInfo& info) {
  EnsureConstructCall(info, "Renderable");
  if (!IsInternalConstruction(info)) {
    ThrowType(info.Env(), "Renderable cannot be constructed directly; finish a builder");
  }
  InitializeEmptyOwner(info, OwnerKind::Renderable);
  return info.This();
}

Napi::Value ConsoleConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Console)) {
    return info.This();
  }
  if (info.Length() != 0) {
    ThrowType(info.Env(), "Console: constructor takes no arguments");
  }
  return CreateOwnerChecked<RichConsole>(
      info.Env(), OwnerKind::Console, "Console",
      [](RichConsole** output) { return rich_console_new(output); });
}

Napi::Value ConsoleSetForceTerminal(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Console, "Console.setForceTerminal");
  CheckResult(info.Env(),
              rich_console_set_force_terminal(OwnerPointer<RichConsole>(owner),
                                              BoolArg(info[0], "Console.setForceTerminal")),
              "Console.setForceTerminal");
  return info.This();
}

Napi::Value ConsoleSetSize(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Console, "Console.setSize");
  const std::uint32_t width = Uint32Arg(info[0], "Console.setSize width");
  const std::uint32_t height = Uint32Arg(info[1], "Console.setSize height");
  CheckResult(info.Env(), rich_console_set_size(OwnerPointer<RichConsole>(owner), width, height),
              "Console.setSize");
  return info.This();
}

Napi::Value ConsoleSetColorSystem(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Console, "Console.setColorSystem");
  CheckResult(info.Env(),
              rich_console_set_color_system(OwnerPointer<RichConsole>(owner),
                                            Uint32Arg(info[0], "Console.setColorSystem")),
              "Console.setColorSystem");
  return info.This();
}

Napi::Value ConsoleSetMarkupEnabled(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Console, "Console.setMarkupEnabled");
  CheckResult(info.Env(),
              rich_console_set_markup_enabled(OwnerPointer<RichConsole>(owner),
                                              BoolArg(info[0], "Console.setMarkupEnabled")),
              "Console.setMarkupEnabled");
  return info.This();
}

Napi::Value ConsoleSetEmojiEnabled(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Console, "Console.setEmojiEnabled");
  CheckResult(info.Env(),
              rich_console_set_emoji_enabled(OwnerPointer<RichConsole>(owner),
                                             BoolArg(info[0], "Console.setEmojiEnabled")),
              "Console.setEmojiEnabled");
  return info.This();
}

Napi::Value ConsoleWidth(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Console, "Console.width");
  std::uint32_t width = 0;
  CheckResult(info.Env(), rich_console_width(OwnerPointer<RichConsole>(owner), &width),
              "Console.width");
  return Napi::Number::New(info.Env(), width);
}

Napi::Value ConsoleRenderMarkup(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Console, "Console.renderMarkup");
  Utf8Arg markup(info[0], "Console.renderMarkup");
  BufferGuard buffer;
  CheckResult(info.Env(),
              rich_console_render_markup(OwnerPointer<RichConsole>(owner), markup.native(),
                                         buffer.output()),
              "Console.renderMarkup");
  return BufferToString(info.Env(), buffer, "Console.renderMarkup");
}

Napi::Value ConsoleRender(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Console, "Console.render");
  Owner* renderable = UnwrapOwner(info.Env(), info[0], OwnerKind::Renderable, "Console.render");
  BufferGuard buffer;
  const RichResult result = rich_console_render(
      OwnerPointer<RichConsole>(owner), OwnerPointer<RichRenderable>(renderable), buffer.output());
  if (RestorePendingException(info.Env()))
    return info.Env().Undefined();
  CheckResult(info.Env(), result, "Console.render");
  return BufferToString(info.Env(), buffer, "Console.render");
}

Napi::Value TextConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Text)) {
    return info.This();
  }
  Utf8Arg text(info[0], "Text");
  return CreateOwnerChecked<RichText>(info.Env(), OwnerKind::Text, "Text", [&](RichText** output) {
    return rich_text_new(text.native(), output);
  });
}

Napi::Value TextFromMarkup(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  EnsureAbi(env);
  Owner* console = UnwrapOwner(env, info[0], OwnerKind::Console, "Text.fromMarkup");
  Utf8Arg markup(info[1], "Text.fromMarkup");
  return CreateOwnerChecked<RichText>(
      env, OwnerKind::Text, "Text.fromMarkup", [&](RichText** output) {
        return rich_text_new_markup(OwnerPointer<RichConsole>(console), markup.native(), output);
      });
}

Napi::Value TextSetStyle(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Text, "Text.setStyle");
  Utf8Arg style(info[0], "Text.setStyle");
  CheckResult(info.Env(), rich_text_set_style(OwnerPointer<RichText>(owner), style.native()),
              "Text.setStyle");
  return info.This();
}

Napi::Value TextFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichText, rich_text_finish>(info, OwnerKind::Text, "Text.finish");
}

Napi::Value StyleConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Style)) {
    return info.This();
  }
  Utf8Arg style(info[0], "Style");
  return CreateOwnerChecked<RichStyle>(
      info.Env(), OwnerKind::Style, "Style",
      [&](RichStyle** output) { return rich_style_parse(style.native(), output); });
}

#define STRING_SETTER(callback_name, kind_value, native_type, native_function, operation)          \
  Napi::Value callback_name(const Napi::CallbackInfo& info) {                                      \
    Owner* owner = ThisOwner(info, kind_value, operation);                                         \
    Utf8Arg input(info[0], operation);                                                             \
    CheckResult(info.Env(), native_function(OwnerPointer<native_type>(owner), input.native()),     \
                operation);                                                                        \
    return info.This();                                                                            \
  }

#define BOOL_SETTER(callback_name, kind_value, native_type, native_function, operation)            \
  Napi::Value callback_name(const Napi::CallbackInfo& info) {                                      \
    Owner* owner = ThisOwner(info, kind_value, operation);                                         \
    CheckResult(info.Env(),                                                                        \
                native_function(OwnerPointer<native_type>(owner), BoolArg(info[0], operation)),    \
                operation);                                                                        \
    return info.This();                                                                            \
  }

#define U32_SETTER(callback_name, kind_value, native_type, native_function, operation)             \
  Napi::Value callback_name(const Napi::CallbackInfo& info) {                                      \
    Owner* owner = ThisOwner(info, kind_value, operation);                                         \
    CheckResult(info.Env(),                                                                        \
                native_function(OwnerPointer<native_type>(owner), Uint32Arg(info[0], operation)),  \
                operation);                                                                        \
    return info.This();                                                                            \
  }

Napi::Value SyntaxConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Syntax)) {
    return info.This();
  }
  Utf8Arg code(info[0], "Syntax code");
  Utf8Arg lexer(info[1], "Syntax lexer");
  return CreateOwnerChecked<RichSyntax>(
      info.Env(), OwnerKind::Syntax, "Syntax",
      [&](RichSyntax** output) { return rich_syntax_new(code.native(), lexer.native(), output); });
}

Napi::Value SyntaxFromPath(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  EnsureAbi(env);
  Utf8Arg path(info[0], "Syntax.fromPath");
  return CreateOwnerChecked<RichSyntax>(
      env, OwnerKind::Syntax, "Syntax.fromPath",
      [&](RichSyntax** output) { return rich_syntax_from_path(path.native(), output); });
}

STRING_SETTER(SyntaxSetTheme, OwnerKind::Syntax, RichSyntax, rich_syntax_set_theme,
              "Syntax.setTheme")
BOOL_SETTER(SyntaxSetLineNumbers, OwnerKind::Syntax, RichSyntax, rich_syntax_set_line_numbers,
            "Syntax.setLineNumbers")
BOOL_SETTER(SyntaxSetWordWrap, OwnerKind::Syntax, RichSyntax, rich_syntax_set_word_wrap,
            "Syntax.setWordWrap")
BOOL_SETTER(SyntaxSetIndentGuides, OwnerKind::Syntax, RichSyntax, rich_syntax_set_indent_guides,
            "Syntax.setIndentGuides")

Napi::Value SyntaxSetLineRange(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Syntax, "Syntax.setLineRange");
  const bool has_start = !info[0].IsNull();
  const bool has_end = !info[1].IsNull();
  const std::uint32_t start = has_start ? Uint32Arg(info[0], "Syntax.setLineRange start") : 0;
  const std::uint32_t end = has_end ? Uint32Arg(info[1], "Syntax.setLineRange end") : 0;
  CheckResult(info.Env(),
              rich_syntax_set_line_range(OwnerPointer<RichSyntax>(owner),
                                         has_start ? UINT8_C(1) : UINT8_C(0), start,
                                         has_end ? UINT8_C(1) : UINT8_C(0), end),
              "Syntax.setLineRange");
  return info.This();
}

Napi::Value SyntaxFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichSyntax, rich_syntax_finish>(info, OwnerKind::Syntax, "Syntax.finish");
}

Napi::Value PrettyConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Pretty))
    return info.This();
  Utf8Arg representation(info[0], "Pretty");
  return CreateOwnerChecked<RichPretty>(
      info.Env(), OwnerKind::Pretty, "Pretty",
      [&](RichPretty** output) { return rich_pretty_new(representation.native(), output); });
}

BOOL_SETTER(PrettySetIndentGuides, OwnerKind::Pretty, RichPretty, rich_pretty_set_indent_guides,
            "Pretty.setIndentGuides")

Napi::Value PrettyFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichPretty, rich_pretty_finish>(info, OwnerKind::Pretty, "Pretty.finish");
}

Napi::Value CustomRenderable(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  EnsureAbi(env);
  Napi::Object renderer = ObjectArg(info[0], "customRenderable");
  RejectUnknownKeys(renderer, {"render", "measure"}, "customRenderable");
  Napi::Value render_value = renderer.Get("render");
  if (!render_value.IsFunction()) {
    ThrowType(env, "customRenderable: render must be a function");
  }
  Napi::Value measure_value = renderer.Get("measure");
  if (!measure_value.IsUndefined() && !measure_value.IsFunction()) {
    ThrowType(env, "customRenderable: measure must be a function when provided");
  }

  auto context = std::make_unique<CustomContext>(CustomContext{
      .env = env,
      .data = Data(env),
      .thread = std::this_thread::get_id(),
      .renderer = nullptr,
      .render = nullptr,
      .measure = nullptr,
  });
  if (napi_create_reference(env, renderer, 1, &context->renderer) != napi_ok ||
      napi_create_reference(env, render_value, 1, &context->render) != napi_ok ||
      (!measure_value.IsUndefined() &&
       napi_create_reference(env, measure_value, 1, &context->measure) != napi_ok)) {
    DestroyCustomContext(context.release(), false);
    throw Napi::Error::New(env, "customRenderable: failed to retain callbacks");
  }
  const RichCustomRenderableVTable vtable{
      .render = CustomRender,
      .measure = context->measure == nullptr ? nullptr : CustomMeasure,
      .drop = CustomDrop,
  };
  RichRenderable* output = nullptr;
  const RichResult result = rich_custom_renderable_new(context.get(), &vtable, &output);
  PendingOwner pending(Data(env), OwnerKind::Renderable, output);
  if (result == RICH_OK || output != nullptr) {
    context.release();
  } else {
    DestroyCustomContext(context.release(), false);
  }
  CheckResult(env, result, "customRenderable");
  return pending.IntoObject(env, "customRenderable");
}

Napi::Value MarkdownConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Markdown)) {
    return info.This();
  }
  Utf8Arg source(info[0], "Markdown");
  return CreateOwnerChecked<RichMarkdown>(
      info.Env(), OwnerKind::Markdown, "Markdown",
      [&](RichMarkdown** output) { return rich_markdown_new(source.native(), output); });
}

STRING_SETTER(MarkdownSetCodeTheme, OwnerKind::Markdown, RichMarkdown, rich_markdown_set_code_theme,
              "Markdown.setCodeTheme")
BOOL_SETTER(MarkdownSetHyperlinks, OwnerKind::Markdown, RichMarkdown, rich_markdown_set_hyperlinks,
            "Markdown.setHyperlinks")
U32_SETTER(MarkdownSetJustify, OwnerKind::Markdown, RichMarkdown, rich_markdown_set_justify,
           "Markdown.setJustify")

Napi::Value MarkdownFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichMarkdown, rich_markdown_finish>(info, OwnerKind::Markdown,
                                                         "Markdown.finish");
}

Napi::Value JsonConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Json)) {
    return info.This();
  }
  Utf8Arg data(info[0], "Json data");
  std::uint32_t indent = 2;
  std::uint8_t highlight = 1;
  std::uint8_t sort_keys = 0;
  if (info.Length() > 1 && !info[1].IsUndefined()) {
    Napi::Object options = ObjectArg(info[1], "Json options");
    RejectUnknownKeys(options, {"indent", "highlight", "sortKeys"}, "Json options");
    if (options.Has("indent")) {
      indent = Uint32Arg(options.Get("indent"), "Json indent");
    }
    if (options.Has("highlight")) {
      highlight = BoolArg(options.Get("highlight"), "Json highlight");
    }
    if (options.Has("sortKeys")) {
      sort_keys = BoolArg(options.Get("sortKeys"), "Json sortKeys");
    }
  }
  return CreateOwnerChecked<RichJson>(info.Env(), OwnerKind::Json, "Json", [&](RichJson** output) {
    return rich_json_new(data.native(), indent, highlight, sort_keys, output);
  });
}

Napi::Value JsonFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichJson, rich_json_finish>(info, OwnerKind::Json, "Json.finish");
}

Napi::Value RuleConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Rule)) {
    return info.This();
  }
  if (info.Length() != 0) {
    ThrowType(info.Env(), "Rule: constructor takes no arguments");
  }
  return CreateOwnerChecked<RichRule>(info.Env(), OwnerKind::Rule, "Rule",
                                      [](RichRule** output) { return rich_rule_new(output); });
}

STRING_SETTER(RuleSetTitle, OwnerKind::Rule, RichRule, rich_rule_set_title, "Rule.setTitle")
STRING_SETTER(RuleSetCharacters, OwnerKind::Rule, RichRule, rich_rule_set_characters,
              "Rule.setCharacters")
STRING_SETTER(RuleSetStyle, OwnerKind::Rule, RichRule, rich_rule_set_style, "Rule.setStyle")
U32_SETTER(RuleSetAlign, OwnerKind::Rule, RichRule, rich_rule_set_align, "Rule.setAlign")

Napi::Value RuleFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichRule, rich_rule_finish>(info, OwnerKind::Rule, "Rule.finish");
}

Napi::Value ColumnsConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Columns)) {
    return info.This();
  }
  if (info.Length() != 0) {
    ThrowType(info.Env(), "Columns: constructor takes no arguments");
  }
  return CreateOwnerChecked<RichColumns>(
      info.Env(), OwnerKind::Columns, "Columns",
      [](RichColumns** output) { return rich_columns_new(output); });
}

Napi::Value ColumnsAdd(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Columns, "Columns.add");
  Owner* renderable = UnwrapOwner(info.Env(), info[0], OwnerKind::Renderable, "Columns.add");
  CheckResult(
      info.Env(),
      rich_columns_add(OwnerPointer<RichColumns>(owner), OwnerPointer<RichRenderable>(renderable)),
      "Columns.add");
  return info.This();
}

Napi::Value ColumnsAddString(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Columns, "Columns.addString");
  Utf8Arg text(info[0], "Columns.addString");
  CheckResult(info.Env(), rich_columns_add_str(OwnerPointer<RichColumns>(owner), text.native()),
              "Columns.addString");
  return info.This();
}

BOOL_SETTER(ColumnsSetEqual, OwnerKind::Columns, RichColumns, rich_columns_set_equal,
            "Columns.setEqual")
BOOL_SETTER(ColumnsSetExpand, OwnerKind::Columns, RichColumns, rich_columns_set_expand,
            "Columns.setExpand")

Napi::Value ColumnsSetPadding(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Columns, "Columns.setPadding");
  const std::uint32_t vertical = Uint32Arg(info[0], "Columns.setPadding vertical");
  const std::uint32_t horizontal = Uint32Arg(info[1], "Columns.setPadding horizontal");
  CheckResult(info.Env(),
              rich_columns_set_padding(OwnerPointer<RichColumns>(owner), vertical, horizontal),
              "Columns.setPadding");
  return info.This();
}

Napi::Value ColumnsFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichColumns, rich_columns_finish>(info, OwnerKind::Columns, "Columns.finish");
}

Napi::Value AlignConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Align)) {
    return info.This();
  }
  Owner* content = UnwrapOwner(info.Env(), info[0], OwnerKind::Renderable, "Align content");
  const std::uint32_t horizontal = Uint32Arg(info[1], "Align horizontal");
  return CreateOwnerChecked<RichAlign>(
      info.Env(), OwnerKind::Align, "Align", [&](RichAlign** output) {
        return rich_align_new(OwnerPointer<RichRenderable>(content), horizontal, output);
      });
}

Napi::Value AlignCenter(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  EnsureAbi(env);
  Owner* content = UnwrapOwner(env, info[0], OwnerKind::Renderable, "Align.center");
  return CreateOwnerChecked<RichAlign>(
      env, OwnerKind::Align, "Align.center", [&](RichAlign** output) {
        return rich_align_center(OwnerPointer<RichRenderable>(content), output);
      });
}

U32_SETTER(AlignSetVertical, OwnerKind::Align, RichAlign, rich_align_set_vertical,
           "Align.setVertical")
U32_SETTER(AlignSetWidth, OwnerKind::Align, RichAlign, rich_align_set_width, "Align.setWidth")

Napi::Value AlignFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichAlign, rich_align_finish>(info, OwnerKind::Align, "Align.finish");
}

Napi::Value PaddingConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Padding)) {
    return info.This();
  }
  Owner* content = UnwrapOwner(info.Env(), info[0], OwnerKind::Renderable, "Padding content");
  const std::uint32_t top = Uint32Arg(info[1], "Padding top");
  const std::uint32_t right = Uint32Arg(info[2], "Padding right");
  const std::uint32_t bottom = Uint32Arg(info[3], "Padding bottom");
  const std::uint32_t left = Uint32Arg(info[4], "Padding left");
  return CreateOwnerChecked<RichPadding>(
      info.Env(), OwnerKind::Padding, "Padding", [&](RichPadding** output) {
        return rich_padding_new(OwnerPointer<RichRenderable>(content), top, right, bottom, left,
                                output);
      });
}

STRING_SETTER(PaddingSetStyle, OwnerKind::Padding, RichPadding, rich_padding_set_style,
              "Padding.setStyle")
BOOL_SETTER(PaddingSetExpand, OwnerKind::Padding, RichPadding, rich_padding_set_expand,
            "Padding.setExpand")

Napi::Value PaddingFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichPadding, rich_padding_finish>(info, OwnerKind::Padding, "Padding.finish");
}

Napi::Value ProgressConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Progress)) {
    return info.This();
  }
  if (info.Length() != 0) {
    ThrowType(info.Env(), "Progress: constructor takes no arguments");
  }
  return CreateOwnerChecked<RichProgress>(
      info.Env(), OwnerKind::Progress, "Progress",
      [](RichProgress** output) { return rich_progress_new(output); });
}

Napi::Value ProgressAddTask(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Progress, "Progress.addTask");
  Utf8Arg description(info[0], "Progress.addTask description");
  const double total = DoubleArg(info[1], "Progress.addTask total");
  std::uint64_t task_id = 0;
  CheckResult(info.Env(),
              rich_progress_add_task(OwnerPointer<RichProgress>(owner), description.native(), total,
                                     &task_id),
              "Progress.addTask");
  return Napi::BigInt::New(info.Env(), task_id);
}

Napi::Value ProgressUpdate(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Progress, "Progress.update");
  const std::uint64_t task_id = Uint64BigIntArg(info[0], "Progress.update taskId");
  const double completed = DoubleArg(info[1], "Progress.update completed");
  CheckResult(info.Env(),
              rich_progress_update(OwnerPointer<RichProgress>(owner), task_id, completed),
              "Progress.update");
  return info.This();
}

Napi::Value ProgressRenderFrame(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Progress, "Progress.renderFrame");
  Owner* console = UnwrapOwner(info.Env(), info[0], OwnerKind::Console, "Progress.renderFrame");
  BufferGuard buffer;
  CheckResult(info.Env(),
              rich_progress_render_frame(OwnerPointer<RichProgress>(owner),
                                         OwnerPointer<RichConsole>(console), buffer.output()),
              "Progress.renderFrame");
  return BufferToString(info.Env(), buffer, "Progress.renderFrame");
}

Napi::Value StatusConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Status)) {
    return info.This();
  }
  Utf8Arg message(info[0], "Status message");
  return CreateOwnerChecked<RichStatus>(
      info.Env(), OwnerKind::Status, "Status",
      [&](RichStatus** output) { return rich_status_new(message.native(), output); });
}

Napi::Value StatusRenderFrame(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Status, "Status.renderFrame");
  Owner* console = UnwrapOwner(info.Env(), info[0], OwnerKind::Console, "Status.renderFrame");
  BufferGuard buffer;
  CheckResult(info.Env(),
              rich_status_render_frame(OwnerPointer<RichStatus>(owner),
                                       OwnerPointer<RichConsole>(console), buffer.output()),
              "Status.renderFrame");
  return BufferToString(info.Env(), buffer, "Status.renderFrame");
}

Napi::Value SpinnerConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Spinner)) {
    return info.This();
  }
  Utf8Arg name(info[0], "Spinner name");
  return CreateOwnerChecked<RichSpinner>(
      info.Env(), OwnerKind::Spinner, "Spinner",
      [&](RichSpinner** output) { return rich_spinner_new(name.native(), output); });
}

Napi::Value SpinnerRenderFrame(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Spinner, "Spinner.renderFrame");
  Owner* console = UnwrapOwner(info.Env(), info[0], OwnerKind::Console, "Spinner.renderFrame");
  const double time = DoubleArg(info[1], "Spinner.renderFrame timeSeconds");
  BufferGuard buffer;
  CheckResult(info.Env(),
              rich_spinner_render_frame(OwnerPointer<RichSpinner>(owner),
                                        OwnerPointer<RichConsole>(console), time, buffer.output()),
              "Spinner.renderFrame");
  return BufferToString(info.Env(), buffer, "Spinner.renderFrame");
}

Napi::Value PanelConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Panel)) {
    return info.This();
  }
  Owner* content = UnwrapOwner(info.Env(), info[0], OwnerKind::Renderable, "Panel content");
  return CreateOwnerChecked<RichPanel>(
      info.Env(), OwnerKind::Panel, "Panel", [&](RichPanel** output) {
        return rich_panel_new(OwnerPointer<RichRenderable>(content), output);
      });
}

STRING_SETTER(PanelSetTitle, OwnerKind::Panel, RichPanel, rich_panel_set_title, "Panel.setTitle")
STRING_SETTER(PanelSetSubtitle, OwnerKind::Panel, RichPanel, rich_panel_set_subtitle,
              "Panel.setSubtitle")
U32_SETTER(PanelSetBox, OwnerKind::Panel, RichPanel, rich_panel_set_box, "Panel.setBox")
BOOL_SETTER(PanelSetExpand, OwnerKind::Panel, RichPanel, rich_panel_set_expand, "Panel.setExpand")
U32_SETTER(PanelSetWidth, OwnerKind::Panel, RichPanel, rich_panel_set_width, "Panel.setWidth")
STRING_SETTER(PanelSetStyle, OwnerKind::Panel, RichPanel, rich_panel_set_style, "Panel.setStyle")
STRING_SETTER(PanelSetBorderStyle, OwnerKind::Panel, RichPanel, rich_panel_set_border_style,
              "Panel.setBorderStyle")

Napi::Value PanelSetPadding(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Panel, "Panel.setPadding");
  const std::uint32_t top = Uint32Arg(info[0], "Panel.setPadding top");
  const std::uint32_t right = Uint32Arg(info[1], "Panel.setPadding right");
  const std::uint32_t bottom = Uint32Arg(info[2], "Panel.setPadding bottom");
  const std::uint32_t left = Uint32Arg(info[3], "Panel.setPadding left");
  CheckResult(info.Env(),
              rich_panel_set_padding(OwnerPointer<RichPanel>(owner), top, right, bottom, left),
              "Panel.setPadding");
  return info.This();
}

Napi::Value PanelFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichPanel, rich_panel_finish>(info, OwnerKind::Panel, "Panel.finish");
}

Napi::Value TableConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Table)) {
    return info.This();
  }
  if (info.Length() != 0) {
    ThrowType(info.Env(), "Table: constructor takes no arguments");
  }
  return CreateOwnerChecked<RichTable>(info.Env(), OwnerKind::Table, "Table",
                                       [](RichTable** output) { return rich_table_new(output); });
}

Napi::Value TableGrid(const Napi::CallbackInfo& info) {
  EnsureAbi(info.Env());
  if (info.Length() != 0)
    ThrowType(info.Env(), "Table.grid: takes no arguments");
  return CreateOwnerChecked<RichTable>(
      info.Env(), OwnerKind::Table, "Table.grid",
      [](RichTable** output) { return rich_table_new_grid(output); });
}

STRING_SETTER(TableSetTitle, OwnerKind::Table, RichTable, rich_table_set_title, "Table.setTitle")
STRING_SETTER(TableSetCaption, OwnerKind::Table, RichTable, rich_table_set_caption,
              "Table.setCaption")
U32_SETTER(TableSetBox, OwnerKind::Table, RichTable, rich_table_set_box, "Table.setBox")
BOOL_SETTER(TableSetPadEdge, OwnerKind::Table, RichTable, rich_table_set_pad_edge,
            "Table.setPadEdge")
BOOL_SETTER(TableSetShowHeader, OwnerKind::Table, RichTable, rich_table_set_show_header,
            "Table.setShowHeader")
BOOL_SETTER(TableSetShowLines, OwnerKind::Table, RichTable, rich_table_set_show_lines,
            "Table.setShowLines")
BOOL_SETTER(TableSetShowEdge, OwnerKind::Table, RichTable, rich_table_set_show_edge,
            "Table.setShowEdge")
BOOL_SETTER(TableSetExpand, OwnerKind::Table, RichTable, rich_table_set_expand, "Table.setExpand")
STRING_SETTER(TableSetStyle, OwnerKind::Table, RichTable, rich_table_set_style, "Table.setStyle")
STRING_SETTER(TableAddColumn, OwnerKind::Table, RichTable, rich_table_add_column, "Table.addColumn")

Napi::Value TableSetTitleMarkup(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Table, "Table.setTitleMarkup");
  Owner* console = UnwrapOwner(info.Env(), info[0], OwnerKind::Console, "Table.setTitleMarkup");
  Utf8Arg markup(info[1], "Table.setTitleMarkup");
  CheckResult(info.Env(),
              rich_table_set_title_markup(OwnerPointer<RichTable>(owner),
                                          OwnerPointer<RichConsole>(console), markup.native()),
              "Table.setTitleMarkup");
  return info.This();
}

Napi::Value TableSetCaptionMarkup(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Table, "Table.setCaptionMarkup");
  Owner* console = UnwrapOwner(info.Env(), info[0], OwnerKind::Console, "Table.setCaptionMarkup");
  Utf8Arg markup(info[1], "Table.setCaptionMarkup");
  CheckResult(info.Env(),
              rich_table_set_caption_markup(OwnerPointer<RichTable>(owner),
                                            OwnerPointer<RichConsole>(console), markup.native()),
              "Table.setCaptionMarkup");
  return info.This();
}

Napi::Value TableSetPadding(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Table, "Table.setPadding");
  const std::uint32_t left = Uint32Arg(info[0], "Table.setPadding left");
  const std::uint32_t right = Uint32Arg(info[1], "Table.setPadding right");
  CheckResult(info.Env(), rich_table_set_padding(OwnerPointer<RichTable>(owner), left, right),
              "Table.setPadding");
  return info.This();
}

Napi::Value TableSetRowStyles(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Table, "Table.setRowStyles");
  if (!info[0].IsArray())
    ThrowType(info.Env(), "Table.setRowStyles: expected an array of strings");
  Napi::Array input = info[0].As<Napi::Array>();
  std::vector<Utf8Arg> storage;
  std::vector<RichStr> styles;
  storage.reserve(input.Length());
  styles.reserve(input.Length());
  for (std::uint32_t index = 0; index < input.Length(); ++index) {
    storage.emplace_back(input.Get(index), "Table.setRowStyles style");
  }
  for (const Utf8Arg& style : storage)
    styles.push_back(style.native());
  CheckResult(info.Env(),
              rich_table_set_row_styles(OwnerPointer<RichTable>(owner),
                                        styles.empty() ? nullptr : styles.data(),
                                        static_cast<std::uint32_t>(styles.size())),
              "Table.setRowStyles");
  return info.This();
}

Napi::Value TableAddColumnOptions(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Table, "Table.addColumnOptions");
  Napi::Object input = ObjectArg(info[0], "Table.addColumnOptions options");
  RejectUnknownKeys(input, {"justify", "vertical", "width", "minWidth", "ratio", "style", "noWrap"},
                    "Table.addColumnOptions options");
  RichTableColumnOptions options{};
  if (input.Has("justify")) {
    options.flags |= RICH_COLUMN_HAS_JUSTIFY;
    options.justify = Uint32Arg(input.Get("justify"), "Table.addColumnOptions justify");
  }
  if (input.Has("vertical")) {
    options.flags |= RICH_COLUMN_HAS_VERTICAL;
    options.vertical = Uint32Arg(input.Get("vertical"), "Table.addColumnOptions vertical");
  }
  if (input.Has("width")) {
    options.flags |= RICH_COLUMN_HAS_WIDTH;
    options.width = Uint32Arg(input.Get("width"), "Table.addColumnOptions width");
  }
  if (input.Has("minWidth")) {
    options.flags |= RICH_COLUMN_HAS_MIN_WIDTH;
    options.min_width = Uint32Arg(input.Get("minWidth"), "Table.addColumnOptions minWidth");
  }
  if (input.Has("ratio")) {
    options.flags |= RICH_COLUMN_HAS_RATIO;
    options.ratio = Uint32Arg(input.Get("ratio"), "Table.addColumnOptions ratio");
  }
  std::unique_ptr<Utf8Arg> style;
  if (input.Has("style")) {
    style = std::make_unique<Utf8Arg>(input.Get("style"), "Table.addColumnOptions style");
    options.style = style->native();
  }
  if (input.Has("noWrap") && BoolArg(input.Get("noWrap"), "Table.addColumnOptions noWrap") != 0) {
    options.flags |= RICH_COLUMN_NO_WRAP;
  }
  const RichRenderable* header = nullptr;
  if (info.Length() > 1 && !info[1].IsUndefined() && !info[1].IsNull()) {
    header = OwnerPointer<RichRenderable>(
        UnwrapOwner(info.Env(), info[1], OwnerKind::Renderable, "Table.addColumnOptions header"));
  }
  CheckResult(info.Env(),
              rich_table_add_column_options(OwnerPointer<RichTable>(owner), header, &options),
              "Table.addColumnOptions");
  return info.This();
}

Napi::Value TableAddColumnRenderable(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Table, "Table.addColumnRenderable");
  Owner* renderable =
      UnwrapOwner(info.Env(), info[0], OwnerKind::Renderable, "Table.addColumnRenderable");
  CheckResult(info.Env(),
              rich_table_add_column_renderable(OwnerPointer<RichTable>(owner),
                                               OwnerPointer<RichRenderable>(renderable)),
              "Table.addColumnRenderable");
  return info.This();
}

Napi::Value TableAddRowStrings(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Table, "Table.addRowStrings");
  if (!info[0].IsArray()) {
    ThrowType(info.Env(), "Table.addRowStrings: expected an array of strings");
  }
  Napi::Array input = info[0].As<Napi::Array>();
  std::vector<Utf8Arg> storage;
  std::vector<RichStr> cells;
  storage.reserve(input.Length());
  cells.reserve(input.Length());
  for (std::uint32_t index = 0; index < input.Length(); ++index) {
    storage.emplace_back(input.Get(index), "Table.addRowStrings cell");
  }
  for (const Utf8Arg& cell : storage) {
    cells.push_back(cell.native());
  }
  CheckResult(info.Env(),
              rich_table_add_row_strs(OwnerPointer<RichTable>(owner), cells.data(),
                                      static_cast<std::uint32_t>(cells.size())),
              "Table.addRowStrings");
  return info.This();
}

Napi::Value TableAddRowRenderables(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Table, "Table.addRowRenderables");
  if (!info[0].IsArray()) {
    ThrowType(info.Env(), "Table.addRowRenderables: expected an array of Renderable");
  }
  Napi::Array input = info[0].As<Napi::Array>();
  std::vector<const RichRenderable*> cells;
  cells.reserve(input.Length());
  for (std::uint32_t index = 0; index < input.Length(); ++index) {
    Owner* renderable = UnwrapOwner(info.Env(), input.Get(index), OwnerKind::Renderable,
                                    "Table.addRowRenderables cell");
    cells.push_back(OwnerPointer<RichRenderable>(renderable));
  }
  CheckResult(info.Env(),
              rich_table_add_row_renderables(OwnerPointer<RichTable>(owner), cells.data(),
                                             static_cast<std::uint32_t>(cells.size())),
              "Table.addRowRenderables");
  return info.This();
}

Napi::Value TableFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichTable, rich_table_finish>(info, OwnerKind::Table, "Table.finish");
}

Napi::Value TreeConstructor(const Napi::CallbackInfo& info) {
  if (BeginConstructor(info, OwnerKind::Tree)) {
    return info.This();
  }
  Utf8Arg label(info[0], "Tree label");
  return CreateOwnerChecked<RichTree>(info.Env(), OwnerKind::Tree, "Tree", [&](RichTree** output) {
    return rich_tree_new(label.native(), output);
  });
}

Napi::Value TreeFromRenderable(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  EnsureAbi(env);
  Owner* label = UnwrapOwner(env, info[0], OwnerKind::Renderable, "Tree.fromRenderable");
  return CreateOwnerChecked<RichTree>(
      env, OwnerKind::Tree, "Tree.fromRenderable", [&](RichTree** output) {
        return rich_tree_new_renderable(OwnerPointer<RichRenderable>(label), output);
      });
}

Napi::Value TreeAdd(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Tree, "Tree.add");
  const std::uint64_t parent_id = Uint64BigIntArg(info[0], "Tree.add parentId");
  Utf8Arg label(info[1], "Tree.add label");
  std::uint64_t output = 0;
  CheckResult(info.Env(),
              rich_tree_add(OwnerPointer<RichTree>(owner), parent_id, label.native(), &output),
              "Tree.add");
  return Napi::BigInt::New(info.Env(), output);
}

Napi::Value TreeAddRenderable(const Napi::CallbackInfo& info) {
  Owner* owner = ThisOwner(info, OwnerKind::Tree, "Tree.addRenderable");
  const std::uint64_t parent_id = Uint64BigIntArg(info[0], "Tree.addRenderable parentId");
  Owner* label =
      UnwrapOwner(info.Env(), info[1], OwnerKind::Renderable, "Tree.addRenderable label");
  std::uint64_t output = 0;
  CheckResult(info.Env(),
              rich_tree_add_renderable(OwnerPointer<RichTree>(owner), parent_id,
                                       OwnerPointer<RichRenderable>(label), &output),
              "Tree.addRenderable");
  return Napi::BigInt::New(info.Env(), output);
}

STRING_SETTER(TreeSetStyle, OwnerKind::Tree, RichTree, rich_tree_set_style, "Tree.setStyle")
STRING_SETTER(TreeSetGuideStyle, OwnerKind::Tree, RichTree, rich_tree_set_guide_style,
              "Tree.setGuideStyle")
BOOL_SETTER(TreeSetHideRoot, OwnerKind::Tree, RichTree, rich_tree_set_hide_root, "Tree.setHideRoot")

Napi::Value TreeFinish(const Napi::CallbackInfo& info) {
  return FinishOwner<RichTree, rich_tree_finish>(info, OwnerKind::Tree, "Tree.finish");
}

Napi::Value TestResult(const Napi::CallbackInfo& info) {
  const std::uint32_t result = Uint32Arg(info[0], "_testResult");
  CheckResult(info.Env(), result, "_testResult");
  return info.Env().Undefined();
}

Napi::Value TestBuffer(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  Utf8Arg mode_arg(info[0], "_testBuffer");
  const RichStr mode_native = mode_arg.native();
  const std::string mode = mode_native.data == nullptr
                               ? std::string()
                               : std::string(reinterpret_cast<const char*>(mode_native.data),
                                             static_cast<std::size_t>(mode_native.len));
  if (mode == "stats") {
    return Napi::BigInt::New(env, Data(env)->test_buffer_frees);
  }
  BufferGuard buffer(Data(env));
  if (mode == "invalidLayout") {
    buffer.output()->data = nullptr;
    buffer.output()->len = 1;
    return BufferToString(env, buffer, "_testBuffer invalidLayout");
  }
  CheckResult(env, rich_ffi_version(buffer.output()), "_testBuffer allocate");
  if (mode == "nativeError") {
    CheckResult(env, RICH_ERROR_INVALID_ARGUMENT, "_testBuffer nativeError");
  }
  if (mode == "throw") {
    throw Napi::Error::New(env, "injected JS allocation-path exception");
  }
  if (mode == "unknownResult") {
    CheckResult(env, UINT32_C(0xffffffff), "_testBuffer unknownResult");
  }
  if (mode != "success") {
    ThrowType(env, "_testBuffer: unknown mode");
  }
  return BufferToString(env, buffer, "_testBuffer success");
}

Napi::Value TestHandle(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  Utf8Arg mode_arg(info[0], "_testHandle");
  const RichStr mode_native = mode_arg.native();
  const std::string mode = mode_native.data == nullptr
                               ? std::string()
                               : std::string(reinterpret_cast<const char*>(mode_native.data),
                                             static_cast<std::size_t>(mode_native.len));
  AddonData* data = Data(env);
  if (mode == "stats") {
    return Napi::BigInt::New(env, data->test_owner_frees);
  }
  if (mode == "nullSuccess") {
    return CreateOwnerChecked<RichConsole>(
        env, OwnerKind::Console, "_testHandle nullSuccess",
        [](RichConsole**) { return static_cast<RichResult>(RICH_OK); });
  }
  if (mode == "nonNullError") {
    return CreateOwnerChecked<RichConsole>(
        env, OwnerKind::Console, "_testHandle nonNullError", [](RichConsole** output) {
          const RichResult result = rich_console_new(output);
          return result == RICH_OK ? static_cast<RichResult>(RICH_ERROR_INVALID_ARGUMENT) : result;
        });
  }
  if (mode == "allocationThrow") {
    data->test_fail_owner_allocation = true;
    try {
      return CreateOwnerChecked<RichConsole>(
          env, OwnerKind::Console, "_testHandle allocationThrow",
          [](RichConsole** output) { return rich_console_new(output); });
    } catch (...) {
      data->test_fail_owner_allocation = false;
      throw;
    }
  }
  if (mode == "armAllocationThrow") {
    data->test_fail_owner_allocation = true;
    return env.Undefined();
  }
  ThrowType(env, "_testHandle: unknown mode");
}

Napi::Value TestFinishFailure(const Napi::CallbackInfo& info) {
  Owner* owner = UnwrapOwner(info.Env(), info[0], OwnerKind::Text, "_testFinishFailure");
  RichText* before = OwnerPointer<RichText>(owner);
  RichText* argument = before;
  const RichResult result = rich_text_finish(&argument, nullptr);
  Napi::Object output = Napi::Object::New(info.Env());
  output.Set("code", Napi::Number::New(info.Env(), result));
  output.Set("retained",
             Napi::Boolean::New(info.Env(), argument == before && owner->pointer == before));
  return output;
}

RichResult NewTestRenderable(RichRenderable** output) {
  static constexpr std::uint8_t kText[]{'f', 'a', 'u', 'l', 't'};
  RichText* text = nullptr;
  RichResult result = rich_text_new(RichStr{kText, sizeof(kText)}, &text);
  if (result != RICH_OK) {
    return result;
  }
  result = rich_text_finish(&text, output);
  if (text != nullptr) {
    rich_text_free(text);
  }
  return result;
}

Napi::Value TestFinishMatrix(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  Owner* owner = UnwrapOwner(env, info[0], OwnerKind::Text, "_testFinishMatrix");
  Utf8Arg input_mode_arg(info[1], "_testFinishMatrix input mode");
  const RichStr input_mode_native = input_mode_arg.native();
  const std::string input_mode(reinterpret_cast<const char*>(input_mode_native.data),
                               static_cast<std::size_t>(input_mode_native.len));
  const bool with_output = BoolArg(info[2], "_testFinishMatrix output") != 0;
  const bool return_ok = BoolArg(info[3], "_testFinishMatrix result") != 0;
  std::string alias_target = "none";
  if (info.Length() > 4) {
    Utf8Arg alias_target_arg(info[4], "_testFinishMatrix alias target");
    const RichStr alias_target_native = alias_target_arg.native();
    alias_target.assign(reinterpret_cast<const char*>(alias_target_native.data),
                        static_cast<std::size_t>(alias_target_native.len));
  }
  if (input_mode != "original" && input_mode != "null" && input_mode != "replacement") {
    ThrowType(env, "_testFinishMatrix: unknown input mode");
  }
  if (alias_target != "none" && alias_target != "input") {
    ThrowType(env, "_testFinishMatrix: unknown alias target");
  }
  if (alias_target != "none" && !with_output) {
    ThrowType(env, "_testFinishMatrix: alias requires output");
  }
  if (alias_target == "input" && input_mode == "null") {
    ThrowType(env, "_testFinishMatrix: input alias requires a live post-call input");
  }

  return FinishOwnerChecked<RichText>(
      env, owner, OwnerKind::Text, "_testFinishMatrix",
      [&](RichText** input, RichRenderable** output) {
        static constexpr std::uint8_t kReplacement[]{'r', 'e', 'p', 'l', 'a', 'c', 'e'};
        if (input_mode == "null") {
          rich_text_free(*input);
          *input = nullptr;
        }
        if (input_mode == "replacement") {
          RichText* replacement = nullptr;
          const RichResult replacement_result =
              rich_text_new(RichStr{kReplacement, sizeof(kReplacement)}, &replacement);
          if (replacement_result != RICH_OK) {
            return replacement_result;
          }
          rich_text_free(*input);
          *input = replacement;
        }
        if (with_output) {
          if (alias_target == "input") {
            *output = reinterpret_cast<RichRenderable*>(*input);
          } else {
            const RichResult output_result = NewTestRenderable(output);
            if (output_result != RICH_OK) {
              return output_result;
            }
          }
        }
        return static_cast<RichResult>(return_ok ? RICH_OK : RICH_ERROR_INVALID_ARGUMENT);
      });
}

Napi::Value TestFinishAddressReuse(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  RichRenderable* reused_output = nullptr;
  CheckResult(env, NewTestRenderable(&reused_output), "_testFinishAddressReuse allocate");

  // Model a consumed builder whose storage address was immediately reused for a
  // genuine RichRenderable. The returned input receipt is NULL, so only the
  // Renderable owns this address and its real destructor remains authoritative.
  Owner synthetic_owner{.kind = OwnerKind::Text,
                        .pointer = reused_output,
                        .free_pointer = FreeFunction(OwnerKind::Text)};
  return FinishOwnerChecked<RichText>(env, &synthetic_owner, OwnerKind::Text,
                                      "_testFinishAddressReuse",
                                      [reused_output](RichText** input, RichRenderable** output) {
                                        *input = nullptr;
                                        *output = reused_output;
                                        return static_cast<RichResult>(RICH_OK);
                                      });
}

Napi::Value TestCustomStats(const Napi::CallbackInfo& info) {
  Napi::Object stats = Napi::Object::New(info.Env());
  stats.Set("drops", Napi::BigInt::New(info.Env(), Data(info.Env())->test_custom_drops));
  stats.Set("renders", Napi::BigInt::New(info.Env(), Data(info.Env())->test_custom_renders));
  return stats;
}

Napi::Object Constants(Napi::Env env) {
  Napi::Object constants = Napi::Object::New(env);
  const std::array<std::pair<const char*, std::uint32_t>, 46> values{{
      {"ABI_VERSION", RICH_FFI_ABI_VERSION},
      {"OK", RICH_OK},
      {"ERROR_NULL_ARGUMENT", RICH_ERROR_NULL_ARGUMENT},
      {"ERROR_INVALID_UTF8", RICH_ERROR_INVALID_UTF8},
      {"ERROR_INVALID_ARGUMENT", RICH_ERROR_INVALID_ARGUMENT},
      {"ERROR_INVALID_STATE", RICH_ERROR_INVALID_STATE},
      {"ERROR_IO", RICH_ERROR_IO},
      {"ERROR_RENDER", RICH_ERROR_RENDER},
      {"ERROR_PANIC", RICH_ERROR_PANIC},
      {"COLOR_NONE", RICH_COLOR_NONE},
      {"COLOR_STANDARD", RICH_COLOR_STANDARD},
      {"COLOR_EIGHT_BIT", RICH_COLOR_EIGHT_BIT},
      {"COLOR_TRUECOLOR", RICH_COLOR_TRUECOLOR},
      {"COLOR_WINDOWS", RICH_COLOR_WINDOWS},
      {"ALIGN_LEFT", RICH_ALIGN_LEFT},
      {"ALIGN_CENTER", RICH_ALIGN_CENTER},
      {"ALIGN_RIGHT", RICH_ALIGN_RIGHT},
      {"VERTICAL_TOP", RICH_VERTICAL_TOP},
      {"VERTICAL_MIDDLE", RICH_VERTICAL_MIDDLE},
      {"VERTICAL_BOTTOM", RICH_VERTICAL_BOTTOM},
      {"JUSTIFY_DEFAULT", RICH_JUSTIFY_DEFAULT},
      {"JUSTIFY_LEFT", RICH_JUSTIFY_LEFT},
      {"JUSTIFY_CENTER", RICH_JUSTIFY_CENTER},
      {"JUSTIFY_RIGHT", RICH_JUSTIFY_RIGHT},
      {"JUSTIFY_FULL", RICH_JUSTIFY_FULL},
      {"BOX_ROUNDED", RICH_BOX_ROUNDED},
      {"BOX_HEAVY", RICH_BOX_HEAVY},
      {"BOX_DOUBLE", RICH_BOX_DOUBLE},
      {"BOX_ASCII", RICH_BOX_ASCII},
      {"BOX_MINIMAL", RICH_BOX_MINIMAL},
      {"BOX_SQUARE", RICH_BOX_SQUARE},
      {"BOX_SIMPLE", RICH_BOX_SIMPLE},
      {"BOX_HEAVY_HEAD", RICH_BOX_HEAVY_HEAD},
      {"BOX_HEAVY_EDGE", RICH_BOX_HEAVY_EDGE},
      {"BOX_DOUBLE_EDGE", RICH_BOX_DOUBLE_EDGE},
      {"BOX_HORIZONTALS", RICH_BOX_HORIZONTALS},
      {"BOX_MINIMAL_HEAVY_HEAD", RICH_BOX_MINIMAL_HEAVY_HEAD},
      {"BOX_SIMPLE_HEAVY", RICH_BOX_SIMPLE_HEAVY},
      {"BOX_MARKDOWN", RICH_BOX_MARKDOWN},
      {"BOX_NONE", RICH_BOX_NONE},
      {"TREE_ROOT_ID", RICH_TREE_ROOT_ID},
      {"ABI_MISMATCH", kAbiMismatch},
      {"UINT32_MAX", UINT32_MAX},
      {"RESULT_SIZE", sizeof(RichResult)},
      {"STR_SIZE", sizeof(RichStr)},
      {"BUFFER_SIZE", sizeof(RichBuffer)},
  }};
  for (const auto& [name, value] : values) {
    constants.Set(name, Napi::Number::New(env, value));
  }
  return constants;
}

Napi::Object Init(Napi::Env env, Napi::Object exports) {
  auto* data = new AddonData();
  env.SetInstanceData(data);
  data->creator_thread = std::this_thread::get_id();
  data->secret = Napi::Persistent<Napi::Value>(Napi::Symbol::New(env, "rich-rs internal"));

  exports.Set("_configure", Napi::Function::New(env, Configure));
  exports.Set("ensureAbi", Napi::Function::New(env, EnsureAbiCallback));
  exports.Set("version", Napi::Function::New(env, VersionCallback));
  exports.Set("_constants", Constants(env));
  exports.Set("_testResult", Napi::Function::New(env, TestResult));
  exports.Set("_testBuffer", Napi::Function::New(env, TestBuffer));
  exports.Set("_testHandle", Napi::Function::New(env, TestHandle));
  exports.Set("_testFinishFailure", Napi::Function::New(env, TestFinishFailure));
  exports.Set("_testFinishMatrix", Napi::Function::New(env, TestFinishMatrix));
  exports.Set("_testFinishAddressReuse", Napi::Function::New(env, TestFinishAddressReuse));
  exports.Set("_testCustomStats", Napi::Function::New(env, TestCustomStats));

  Napi::Function render_options =
      Napi::Function::New(env, RenderOptionsConstructor, "RenderOptions");
  data->render_options_constructor = Napi::Persistent(render_options);
  exports.Set("RenderOptions", render_options);
  Napi::Function measurement = Napi::Function::New(env, MeasurementConstructor, "Measurement");
  data->measurement_constructor = Napi::Persistent(measurement);
  exports.Set("Measurement", measurement);
  Napi::Function segment_sink = Napi::Function::New(env, SegmentSinkConstructor, "SegmentSink");
  SetPrototypeMethod(env, segment_sink, "push", SegmentSinkPush);
  SetPrototypeMethod(env, segment_sink, "line", SegmentSinkLine);
  data->segment_sink_constructor = Napi::Persistent(segment_sink);
  exports.Set("SegmentSink", segment_sink);
  exports.Set("customRenderable", Napi::Function::New(env, CustomRenderable));

  RegisterClass(env, exports, OwnerKind::Renderable, RenderableConstructor, {});
  Napi::Function console = RegisterClass(env, exports, OwnerKind::Console, ConsoleConstructor,
                                         {{"setForceTerminal", ConsoleSetForceTerminal},
                                          {"setSize", ConsoleSetSize},
                                          {"setColorSystem", ConsoleSetColorSystem},
                                          {"setMarkupEnabled", ConsoleSetMarkupEnabled},
                                          {"setEmojiEnabled", ConsoleSetEmojiEnabled},
                                          {"width", ConsoleWidth},
                                          {"renderMarkup", ConsoleRenderMarkup},
                                          {"render", ConsoleRender}});
  (void)console;
  Napi::Function text = RegisterClass(env, exports, OwnerKind::Text, TextConstructor,
                                      {{"setStyle", TextSetStyle}, {"finish", TextFinish}});
  SetStaticMethod(env, text, "fromMarkup", TextFromMarkup);
  RegisterClass(env, exports, OwnerKind::Style, StyleConstructor, {});

  Napi::Function table = RegisterClass(env, exports, OwnerKind::Table, TableConstructor,
                                       {{"setTitle", TableSetTitle},
                                        {"setCaption", TableSetCaption},
                                        {"setTitleMarkup", TableSetTitleMarkup},
                                        {"setCaptionMarkup", TableSetCaptionMarkup},
                                        {"setBox", TableSetBox},
                                        {"setPadEdge", TableSetPadEdge},
                                        {"setRowStyles", TableSetRowStyles},
                                        {"setShowHeader", TableSetShowHeader},
                                        {"setShowLines", TableSetShowLines},
                                        {"setShowEdge", TableSetShowEdge},
                                        {"setExpand", TableSetExpand},
                                        {"setPadding", TableSetPadding},
                                        {"setStyle", TableSetStyle},
                                        {"addColumn", TableAddColumn},
                                        {"addColumnRenderable", TableAddColumnRenderable},
                                        {"addColumnOptions", TableAddColumnOptions},
                                        {"addRowStrings", TableAddRowStrings},
                                        {"addRowRenderables", TableAddRowRenderables},
                                        {"finish", TableFinish}});
  SetStaticMethod(env, table, "grid", TableGrid);
  RegisterClass(env, exports, OwnerKind::Panel, PanelConstructor,
                {{"setTitle", PanelSetTitle},
                 {"setSubtitle", PanelSetSubtitle},
                 {"setBox", PanelSetBox},
                 {"setExpand", PanelSetExpand},
                 {"setWidth", PanelSetWidth},
                 {"setPadding", PanelSetPadding},
                 {"setStyle", PanelSetStyle},
                 {"setBorderStyle", PanelSetBorderStyle},
                 {"finish", PanelFinish}});
  Napi::Function tree = RegisterClass(env, exports, OwnerKind::Tree, TreeConstructor,
                                      {{"add", TreeAdd},
                                       {"addRenderable", TreeAddRenderable},
                                       {"setStyle", TreeSetStyle},
                                       {"setGuideStyle", TreeSetGuideStyle},
                                       {"setHideRoot", TreeSetHideRoot},
                                       {"finish", TreeFinish}});
  SetStaticMethod(env, tree, "fromRenderable", TreeFromRenderable);
  RegisterClass(env, exports, OwnerKind::Rule, RuleConstructor,
                {{"setTitle", RuleSetTitle},
                 {"setCharacters", RuleSetCharacters},
                 {"setStyle", RuleSetStyle},
                 {"setAlign", RuleSetAlign},
                 {"finish", RuleFinish}});
  RegisterClass(env, exports, OwnerKind::Columns, ColumnsConstructor,
                {{"add", ColumnsAdd},
                 {"addString", ColumnsAddString},
                 {"setEqual", ColumnsSetEqual},
                 {"setExpand", ColumnsSetExpand},
                 {"setPadding", ColumnsSetPadding},
                 {"finish", ColumnsFinish}});
  Napi::Function align = RegisterClass(
      env, exports, OwnerKind::Align, AlignConstructor,
      {{"setVertical", AlignSetVertical}, {"setWidth", AlignSetWidth}, {"finish", AlignFinish}});
  SetStaticMethod(env, align, "center", AlignCenter);
  RegisterClass(
      env, exports, OwnerKind::Padding, PaddingConstructor,
      {{"setStyle", PaddingSetStyle}, {"setExpand", PaddingSetExpand}, {"finish", PaddingFinish}});
  Napi::Function syntax = RegisterClass(env, exports, OwnerKind::Syntax, SyntaxConstructor,
                                        {{"setTheme", SyntaxSetTheme},
                                         {"setLineNumbers", SyntaxSetLineNumbers},
                                         {"setWordWrap", SyntaxSetWordWrap},
                                         {"setIndentGuides", SyntaxSetIndentGuides},
                                         {"setLineRange", SyntaxSetLineRange},
                                         {"finish", SyntaxFinish}});
  SetStaticMethod(env, syntax, "fromPath", SyntaxFromPath);
  RegisterClass(env, exports, OwnerKind::Pretty, PrettyConstructor,
                {{"setIndentGuides", PrettySetIndentGuides}, {"finish", PrettyFinish}});
  RegisterClass(env, exports, OwnerKind::Markdown, MarkdownConstructor,
                {{"setCodeTheme", MarkdownSetCodeTheme},
                 {"setHyperlinks", MarkdownSetHyperlinks},
                 {"setJustify", MarkdownSetJustify},
                 {"finish", MarkdownFinish}});
  RegisterClass(env, exports, OwnerKind::Json, JsonConstructor, {{"finish", JsonFinish}});
  RegisterClass(env, exports, OwnerKind::Progress, ProgressConstructor,
                {{"addTask", ProgressAddTask},
                 {"update", ProgressUpdate},
                 {"renderFrame", ProgressRenderFrame}});
  RegisterClass(env, exports, OwnerKind::Status, StatusConstructor,
                {{"renderFrame", StatusRenderFrame}});
  RegisterClass(env, exports, OwnerKind::Spinner, SpinnerConstructor,
                {{"renderFrame", SpinnerRenderFrame}});
  return exports;
}

#undef STRING_SETTER
#undef BOOL_SETTER
#undef U32_SETTER
#undef CLOSE_CALLBACK

} // namespace
} // namespace rich_node

Napi::Object InitRichNative(Napi::Env env, Napi::Object exports) {
  return rich_node::Init(env, exports);
}

NODE_API_MODULE(rich_native, InitRichNative)
