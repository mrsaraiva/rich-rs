#include "common.h"

#include <algorithm>
#include <cmath>
#include <cstring>
#include <sstream>
#include <utility>

namespace rich_node {
namespace {

constexpr auto MakeOwnerTags() {
  std::array<napi_type_tag, static_cast<std::size_t>(OwnerKind::Count)> tags{};
  for (std::size_t index = 0; index < tags.size(); ++index) {
    tags[index] = napi_type_tag{UINT64_C(0x726963682d6e6f64), UINT64_C(0x6500000000000000) + index};
  }
  return tags;
}

constexpr auto kOwnerTags = MakeOwnerTags();

thread_local bool g_in_custom_callback = false;
thread_local napi_env g_pending_exception_env = nullptr;
thread_local napi_ref g_pending_exception = nullptr;

const napi_type_tag* OwnerTag(OwnerKind kind) { return &kOwnerTags.at(KindIndex(kind)); }

void FinalizeOwner(napi_env, void* data, void*) {
  auto* owner = static_cast<Owner*>(data);
  CloseOwner(owner);
  owner->magic = 0;
  delete owner;
}

std::string Message(std::string_view operation, std::string_view detail) {
  std::string message(operation);
  if (!detail.empty()) {
    message.append(": ");
    message.append(detail);
  }
  return message;
}

bool AllowedKey(std::string_view key, std::initializer_list<std::string_view> allowed) {
  return std::find(allowed.begin(), allowed.end(), key) != allowed.end();
}

} // namespace

static_assert(sizeof(std::uint8_t) == 1);
static_assert(sizeof(std::uint32_t) == 4);
static_assert(sizeof(std::uint64_t) == 8);
static_assert(sizeof(RichResult) == sizeof(std::uint32_t));
static_assert(sizeof(RichStr) == sizeof(void*) + sizeof(std::uint64_t));
static_assert(sizeof(RichBuffer) == sizeof(void*) + sizeof(std::uint64_t));
static_assert(offsetof(RichStr, data) == 0);
static_assert(offsetof(RichStr, len) == sizeof(void*));
static_assert(offsetof(RichBuffer, data) == 0);
static_assert(offsetof(RichBuffer, len) == sizeof(void*));
static_assert(sizeof(RichRenderOptions) == 24);
static_assert(offsetof(RichRenderOptions, reserved) == 17);
static_assert(sizeof(RichMeasurement) == 8);
static_assert(sizeof(RichTableColumnOptions) == 40);
static_assert(offsetof(RichTableColumnOptions, style) == 24);
static_assert(RICH_FFI_ABI_VERSION == 2);
static_assert(RICH_BOX_NONE == 14);
static_assert(RICH_COLUMN_HAS_JUSTIFY == (1 << 0));
static_assert(RICH_COLUMN_HAS_VERTICAL == (1 << 1));
static_assert(RICH_COLUMN_HAS_WIDTH == (1 << 2));
static_assert(RICH_COLUMN_HAS_MIN_WIDTH == (1 << 3));
static_assert(RICH_COLUMN_HAS_RATIO == (1 << 4));
static_assert(RICH_COLUMN_NO_WRAP == (1 << 5));
static_assert(RICH_OK == 0);
static_assert(RICH_ERROR_PANIC == 7);

#define RICH_NATIVE_SYMBOL(name) static_assert(std::is_pointer_v<decltype(&name)>);
#include "native_symbols.inc"
#undef RICH_NATIVE_SYMBOL

AddonData::~AddonData() {
  for (auto& constructor : constructors) {
    constructor.Reset();
  }
  secret.Reset();
  rich_error.Reset();
  render_options_constructor.Reset();
  measurement_constructor.Reset();
  segment_sink_constructor.Reset();
}

Utf8Arg::Utf8Arg(const Napi::Value& value, std::string_view operation) {
  if (!value.IsString()) {
    ThrowType(value.Env(), Message(operation, "expected a string"));
  }
  bytes_ = value.As<Napi::String>().Utf8Value();
}

RichStr Utf8Arg::native() const {
  if (bytes_.empty()) {
    return RichStr{nullptr, 0};
  }
  return RichStr{reinterpret_cast<const std::uint8_t*>(bytes_.data()),
                 static_cast<std::uint64_t>(bytes_.size())};
}

BufferGuard::BufferGuard(AddonData* data) : data_(data) {}

BufferGuard::~BufferGuard() {
  if (armed_) {
    rich_buffer_free(&buffer_);
    if (data_ != nullptr) {
      ++data_->test_buffer_frees;
    }
  }
}

RichBuffer* BufferGuard::output() { return &buffer_; }

const RichBuffer& BufferGuard::value() const { return buffer_; }

void BufferGuard::disarm() { armed_ = false; }

PendingOwner::PendingOwner(AddonData* data, OwnerKind kind, void* pointer)
    : data_(data), kind_(kind), pointer_(pointer) {}

PendingOwner::~PendingOwner() {
  if (pointer_ != nullptr) {
    FreeFunction(kind_)(pointer_);
    pointer_ = nullptr;
    if (data_ != nullptr) {
      ++data_->test_owner_frees;
    }
  }
}

void* PendingOwner::Release() {
  void* pointer = pointer_;
  pointer_ = nullptr;
  return pointer;
}

Napi::Object PendingOwner::IntoObject(Napi::Env env, std::string_view operation) {
  if (pointer_ == nullptr) {
    ThrowRich(env, kAbiMismatch, operation, "native constructor returned a null handle");
  }
  if (data_->test_fail_owner_allocation) {
    data_->test_fail_owner_allocation = false;
    throw Napi::Error::New(env, "injected owner-wrapper allocation exception");
  }
  Napi::Object object = data_->constructors.at(KindIndex(kind_)).New({data_->secret.Value()});
  Owner* owner = UnwrapOwner(env, object, kind_, operation, false);
  if (owner->pointer != nullptr) {
    ThrowRich(env, kAbiMismatch, operation, "new owner wrapper was not empty");
  }
  owner->pointer = Release();
  return object;
}

AddonData* Data(Napi::Env env) {
  auto* data = env.GetInstanceData<AddonData>();
  if (data == nullptr) {
    throw Napi::Error::New(env, "rich-rs addon environment is not initialized");
  }
  return data;
}

std::size_t KindIndex(OwnerKind kind) { return static_cast<std::size_t>(kind); }

const char* KindName(OwnerKind kind) {
  constexpr std::array names{"Console", "Renderable", "Text",    "Style",    "Table",   "Panel",
                             "Tree",    "Rule",       "Columns", "Align",    "Padding", "Syntax",
                             "Pretty",  "Markdown",   "Json",    "Progress", "Status",  "Spinner"};
  return names.at(KindIndex(kind));
}

bool IsInternalConstruction(const Napi::CallbackInfo& info) {
  return info.Length() == 1 && Data(info.Env())->secret.Value().StrictEquals(info[0]);
}

void EnsureConstructCall(const Napi::CallbackInfo& info, std::string_view name) {
  if (!info.IsConstructCall()) {
    ThrowType(info.Env(), std::string(name) + " must be called with new");
  }
}

void EnsureAbi(Napi::Env env) {
  EnsureOrdinaryCallAllowed(env, "ensureAbi");
  const std::uint32_t loaded = rich_ffi_abi_version();
  if (loaded != RICH_FFI_ABI_VERSION) {
    std::ostringstream detail;
    detail << "compiled ABI " << RICH_FFI_ABI_VERSION << ", loaded ABI " << loaded;
    ThrowRich(env, kAbiMismatch, "ensureAbi", detail.str());
  }
}

void EnsureOrdinaryCallAllowed(Napi::Env env, std::string_view operation) {
  if (g_in_custom_callback) {
    ThrowRich(env, RICH_ERROR_INVALID_STATE, operation,
              "operation is not allowed during a custom-renderable callback");
  }
}

bool InCustomCallback() { return g_in_custom_callback; }

void EnterCustomCallback() { g_in_custom_callback = true; }

void LeaveCustomCallback() { g_in_custom_callback = false; }

void RecordPendingException(napi_env env, napi_value exception) noexcept {
  if (g_pending_exception != nullptr || exception == nullptr) {
    return;
  }
  napi_ref reference = nullptr;
  if (napi_create_reference(env, exception, 1, &reference) == napi_ok) {
    g_pending_exception_env = env;
    g_pending_exception = reference;
  }
}

void RecordPendingError(napi_env env, const char* message) noexcept {
  napi_value text = nullptr;
  napi_value error = nullptr;
  if (napi_create_string_utf8(env, message, NAPI_AUTO_LENGTH, &text) == napi_ok &&
      napi_create_error(env, nullptr, text, &error) == napi_ok) {
    RecordPendingException(env, error);
  }
}

bool RestorePendingException(napi_env env) noexcept {
  if (g_pending_exception == nullptr) {
    return false;
  }
  napi_ref reference = g_pending_exception;
  napi_env exception_env = g_pending_exception_env;
  g_pending_exception = nullptr;
  g_pending_exception_env = nullptr;
  if (exception_env != env) {
    if (exception_env != nullptr) {
      napi_delete_reference(exception_env, reference);
    }
    napi_throw_error(env, nullptr, "custom-renderable exception belongs to another environment");
    return true;
  }
  napi_value exception = nullptr;
  const napi_status get_status = napi_get_reference_value(env, reference, &exception);
  napi_delete_reference(env, reference);
  if (get_status != napi_ok || exception == nullptr) {
    napi_throw_error(env, nullptr, "failed to restore custom-renderable exception");
    return true;
  }
  napi_throw(env, exception);
  return true;
}

void CheckResult(Napi::Env env, RichResult result, std::string_view operation) {
  if (result == RICH_OK) {
    return;
  }
  switch (result) {
  case RICH_ERROR_NULL_ARGUMENT:
  case RICH_ERROR_INVALID_UTF8:
  case RICH_ERROR_INVALID_ARGUMENT:
  case RICH_ERROR_INVALID_STATE:
  case RICH_ERROR_IO:
  case RICH_ERROR_RENDER:
  case RICH_ERROR_PANIC:
    ThrowRich(env, result, operation);
  default:
    ThrowRich(env, kAbiMismatch, operation, "unknown native result code");
  }
}

void ThrowRich(Napi::Env env, std::uint32_t code, std::string_view operation,
               std::string_view detail) {
  const std::string message = Message(operation, detail);
  AddonData* data = Data(env);
  if (!data->rich_error.IsEmpty()) {
    Napi::Object error =
        data->rich_error.New({Napi::String::New(env, message), Napi::Number::New(env, code)});
    throw Napi::Error(env, error);
  }
  Napi::Error error = Napi::Error::New(env, message);
  error.Value().Set("name", "RichError");
  error.Value().Set("code", Napi::Number::New(env, code));
  throw error;
}

void ThrowType(Napi::Env env, std::string_view message) {
  throw Napi::TypeError::New(env, std::string(message));
}

void ThrowRange(Napi::Env env, std::string_view message) {
  throw Napi::RangeError::New(env, std::string(message));
}

std::uint32_t Uint32Arg(const Napi::Value& value, std::string_view operation) {
  if (!value.IsNumber()) {
    ThrowType(value.Env(), Message(operation, "expected a uint32 Number"));
  }
  const double number = value.As<Napi::Number>().DoubleValue();
  if (!std::isfinite(number) || std::floor(number) != number || number < 0 ||
      number > std::numeric_limits<std::uint32_t>::max()) {
    ThrowRange(value.Env(), Message(operation, "Number is outside uint32 range"));
  }
  return static_cast<std::uint32_t>(number);
}

std::uint64_t Uint64BigIntArg(const Napi::Value& value, std::string_view operation) {
  if (!value.IsBigInt()) {
    ThrowType(value.Env(), Message(operation, "expected a nonnegative bigint"));
  }
  bool lossless = false;
  const std::uint64_t output = value.As<Napi::BigInt>().Uint64Value(&lossless);
  if (!lossless) {
    ThrowRange(value.Env(), Message(operation, "bigint is outside uint64 range"));
  }
  return output;
}

double DoubleArg(const Napi::Value& value, std::string_view operation) {
  if (!value.IsNumber()) {
    ThrowType(value.Env(), Message(operation, "expected a Number"));
  }
  return value.As<Napi::Number>().DoubleValue();
}

std::uint8_t BoolArg(const Napi::Value& value, std::string_view operation) {
  if (!value.IsBoolean()) {
    ThrowType(value.Env(), Message(operation, "expected a boolean"));
  }
  return value.As<Napi::Boolean>().Value() ? UINT8_C(1) : UINT8_C(0);
}

Napi::Object ObjectArg(const Napi::Value& value, std::string_view operation) {
  if (!value.IsObject() || value.IsArray() || value.IsFunction() || value.IsNull()) {
    ThrowType(value.Env(), Message(operation, "expected an options object"));
  }
  return value.As<Napi::Object>();
}

void RejectUnknownKeys(const Napi::Object& object, std::initializer_list<std::string_view> allowed,
                       std::string_view operation) {
  Napi::Array keys = object.GetPropertyNames();
  for (std::uint32_t index = 0; index < keys.Length(); ++index) {
    const std::string key = keys.Get(index).ToString().Utf8Value();
    if (!AllowedKey(key, allowed)) {
      ThrowType(object.Env(), Message(operation, "unknown option '" + key + "'"));
    }
  }
}

Owner* UnwrapOwner(Napi::Env env, const Napi::Value& value, OwnerKind expected,
                   std::string_view operation, bool require_open) {
  EnsureOrdinaryCallAllowed(env, operation);
  if (!value.IsObject()) {
    ThrowType(env, Message(operation, std::string("expected ") + KindName(expected)));
  }
  Napi::Object object = value.As<Napi::Object>();
  AddonData* data = Data(env);
  if (!object.InstanceOf(data->constructors.at(KindIndex(expected)).Value())) {
    ThrowType(env, Message(operation, std::string("expected ") + KindName(expected)));
  }
  bool has_expected_tag = false;
  const napi_status tag_status =
      napi_check_object_type_tag(env, object, OwnerTag(expected), &has_expected_tag);
  if (tag_status != napi_ok || !has_expected_tag) {
    ThrowType(env, Message(operation, std::string("expected ") + KindName(expected)));
  }
  void* raw = nullptr;
  const napi_status status = napi_unwrap(env, object, &raw);
  if (status != napi_ok || raw == nullptr) {
    ThrowType(env, Message(operation, std::string("expected ") + KindName(expected)));
  }
  auto* owner = static_cast<Owner*>(raw);
  if (owner->magic != kOwnerMagic || owner->kind != expected) {
    ThrowType(env, Message(operation, std::string("expected ") + KindName(expected)));
  }
  if (require_open && owner->pointer == nullptr) {
    ThrowRich(env, RICH_ERROR_INVALID_STATE, operation,
              std::string(KindName(expected)) + " is closed or finished");
  }
  return owner;
}

Owner* ThisOwner(const Napi::CallbackInfo& info, OwnerKind expected, std::string_view operation,
                 bool require_open) {
  return UnwrapOwner(info.Env(), info.This(), expected, operation, require_open);
}

void WrapOwner(Napi::Env env, Napi::Object object, Owner* owner) {
  const napi_status tag_status = napi_type_tag_object(env, object, OwnerTag(owner->kind));
  if (tag_status != napi_ok) {
    CloseOwner(owner);
    delete owner;
    throw Napi::Error::New(env, "failed to type-tag private native owner");
  }
  const napi_status wrap_status = napi_wrap(env, object, owner, FinalizeOwner, nullptr, nullptr);
  if (wrap_status != napi_ok) {
    CloseOwner(owner);
    delete owner;
    throw Napi::Error::New(env, "failed to attach private native owner");
  }
}

void CloseOwner(Owner* owner) {
  if (owner != nullptr && owner->magic == kOwnerMagic && owner->pointer != nullptr) {
    owner->free_pointer(owner->pointer);
    owner->pointer = nullptr;
  }
}

Napi::String BufferToString(Napi::Env env, BufferGuard& buffer, std::string_view operation) {
  const RichBuffer& value = buffer.value();
  if ((value.data == nullptr) != (value.len == 0)) {
    ThrowRich(env, kAbiMismatch, operation, "native buffer has invalid pointer/length layout");
  }
  if (value.len > std::numeric_limits<std::size_t>::max()) {
    ThrowRich(env, kAbiMismatch, operation, "native buffer length exceeds size_t");
  }
  const char* bytes = value.data == nullptr ? "" : reinterpret_cast<const char*>(value.data);
  return Napi::String::New(env, bytes, static_cast<std::size_t>(value.len));
}

void (*FreeFunction(OwnerKind kind))(void*) {
  switch (kind) {
  case OwnerKind::Console:
    return FreeErased<RichConsole, rich_console_free>;
  case OwnerKind::Renderable:
    return FreeErased<RichRenderable, rich_renderable_free>;
  case OwnerKind::Text:
    return FreeErased<RichText, rich_text_free>;
  case OwnerKind::Style:
    return FreeErased<RichStyle, rich_style_free>;
  case OwnerKind::Table:
    return FreeErased<RichTable, rich_table_free>;
  case OwnerKind::Panel:
    return FreeErased<RichPanel, rich_panel_free>;
  case OwnerKind::Tree:
    return FreeErased<RichTree, rich_tree_free>;
  case OwnerKind::Rule:
    return FreeErased<RichRule, rich_rule_free>;
  case OwnerKind::Columns:
    return FreeErased<RichColumns, rich_columns_free>;
  case OwnerKind::Align:
    return FreeErased<RichAlign, rich_align_free>;
  case OwnerKind::Padding:
    return FreeErased<RichPadding, rich_padding_free>;
  case OwnerKind::Syntax:
    return FreeErased<RichSyntax, rich_syntax_free>;
  case OwnerKind::Pretty:
    return FreeErased<RichPretty, rich_pretty_free>;
  case OwnerKind::Markdown:
    return FreeErased<RichMarkdown, rich_markdown_free>;
  case OwnerKind::Json:
    return FreeErased<RichJson, rich_json_free>;
  case OwnerKind::Progress:
    return FreeErased<RichProgress, rich_progress_free>;
  case OwnerKind::Status:
    return FreeErased<RichStatus, rich_status_free>;
  case OwnerKind::Spinner:
    return FreeErased<RichSpinner, rich_spinner_free>;
  case OwnerKind::Count:
    break;
  }
  return nullptr;
}

} // namespace rich_node
