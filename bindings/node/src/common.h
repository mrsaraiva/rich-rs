#ifndef RICH_RS_NODE_COMMON_H
#define RICH_RS_NODE_COMMON_H

#include <napi.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <initializer_list>
#include <limits>
#include <string>
#include <string_view>
#include <thread>
#include <type_traits>
#include <utility>

#include "rich.h"

namespace rich_node {

constexpr std::uint32_t kAbiMismatch = 8;
constexpr std::uint64_t kOwnerMagic = UINT64_C(0x524943484e4f4445);

enum class OwnerKind : std::uint32_t {
  Console,
  Renderable,
  Text,
  Style,
  Table,
  Panel,
  Tree,
  Rule,
  Columns,
  Align,
  Padding,
  Syntax,
  Pretty,
  Markdown,
  Json,
  Progress,
  Status,
  Spinner,
  Count,
};

struct Owner {
  std::uint64_t magic{kOwnerMagic};
  OwnerKind kind;
  void* pointer{nullptr};
  void (*free_pointer)(void*){nullptr};
};

struct AddonData {
  std::array<Napi::FunctionReference, static_cast<std::size_t>(OwnerKind::Count)> constructors;
  Napi::Reference<Napi::Value> secret;
  Napi::FunctionReference rich_error;
  std::uint64_t test_buffer_frees{0};
  std::uint64_t test_owner_frees{0};
  std::uint64_t test_custom_drops{0};
  std::uint64_t test_custom_renders{0};
  bool test_fail_owner_allocation{false};
  Napi::FunctionReference render_options_constructor;
  Napi::FunctionReference measurement_constructor;
  Napi::FunctionReference segment_sink_constructor;
  std::thread::id creator_thread;

  ~AddonData();
};

class Utf8Arg {
public:
  Utf8Arg(const Napi::Value& value, std::string_view operation);
  [[nodiscard]] RichStr native() const;

private:
  std::string bytes_;
};

class BufferGuard {
public:
  explicit BufferGuard(AddonData* data = nullptr);
  BufferGuard(const BufferGuard&) = delete;
  BufferGuard& operator=(const BufferGuard&) = delete;
  ~BufferGuard();

  [[nodiscard]] RichBuffer* output();
  [[nodiscard]] const RichBuffer& value() const;
  void disarm();

private:
  RichBuffer buffer_{nullptr, 0};
  AddonData* data_;
  bool armed_{true};
};

class PendingOwner {
public:
  PendingOwner(AddonData* data, OwnerKind kind, void* pointer);
  PendingOwner(const PendingOwner&) = delete;
  PendingOwner& operator=(const PendingOwner&) = delete;
  ~PendingOwner();

  [[nodiscard]] Napi::Object IntoObject(Napi::Env env, std::string_view operation);

private:
  [[nodiscard]] void* Release();

  AddonData* data_;
  OwnerKind kind_;
  void* pointer_;
};

[[nodiscard]] AddonData* Data(Napi::Env env);
[[nodiscard]] std::size_t KindIndex(OwnerKind kind);
[[nodiscard]] const char* KindName(OwnerKind kind);
[[nodiscard]] bool IsInternalConstruction(const Napi::CallbackInfo& info);
void EnsureConstructCall(const Napi::CallbackInfo& info, std::string_view name);
void EnsureAbi(Napi::Env env);
void EnsureOrdinaryCallAllowed(Napi::Env env, std::string_view operation);
[[nodiscard]] bool InCustomCallback();
void EnterCustomCallback();
void LeaveCustomCallback();
void RecordPendingException(napi_env env, napi_value exception) noexcept;
void RecordPendingError(napi_env env, const char* message) noexcept;
[[nodiscard]] bool RestorePendingException(napi_env env) noexcept;
void CheckResult(Napi::Env env, RichResult result, std::string_view operation);
[[noreturn]] void ThrowRich(Napi::Env env, std::uint32_t code, std::string_view operation,
                            std::string_view detail = {});
[[noreturn]] void ThrowType(Napi::Env env, std::string_view message);
[[noreturn]] void ThrowRange(Napi::Env env, std::string_view message);

[[nodiscard]] std::uint32_t Uint32Arg(const Napi::Value& value, std::string_view operation);
[[nodiscard]] std::uint64_t Uint64BigIntArg(const Napi::Value& value, std::string_view operation);
[[nodiscard]] double DoubleArg(const Napi::Value& value, std::string_view operation);
[[nodiscard]] std::uint8_t BoolArg(const Napi::Value& value, std::string_view operation);
[[nodiscard]] Napi::Object ObjectArg(const Napi::Value& value, std::string_view operation);
void RejectUnknownKeys(const Napi::Object& object, std::initializer_list<std::string_view> allowed,
                       std::string_view operation);

[[nodiscard]] Owner* UnwrapOwner(Napi::Env env, const Napi::Value& value, OwnerKind expected,
                                 std::string_view operation, bool require_open = true);
[[nodiscard]] Owner* ThisOwner(const Napi::CallbackInfo& info, OwnerKind expected,
                               std::string_view operation, bool require_open = true);
void WrapOwner(Napi::Env env, Napi::Object object, Owner* owner);
void CloseOwner(Owner* owner);
[[nodiscard]] Napi::String BufferToString(Napi::Env env, BufferGuard& buffer,
                                          std::string_view operation);

template <typename T> [[nodiscard]] T* OwnerPointer(Owner* owner) {
  return static_cast<T*>(owner->pointer);
}

template <typename T, void (*Free)(T*)> void FreeErased(void* pointer) {
  Free(static_cast<T*>(pointer));
}

[[nodiscard]] void (*FreeFunction(OwnerKind kind))(void*);

template <typename Native, typename Factory>
Napi::Object CreateOwnerChecked(Napi::Env env, OwnerKind kind, std::string_view operation,
                                Factory&& factory) {
  Native* output = nullptr;
  const RichResult result = std::forward<Factory>(factory)(&output);
  PendingOwner pending(Data(env), kind, output);
  CheckResult(env, result, operation);
  return pending.IntoObject(env, operation);
}

template <typename Builder, typename Finish>
Napi::Value FinishOwnerChecked(Napi::Env env, Owner* owner, OwnerKind kind,
                               std::string_view operation, Finish&& finish) {
  AddonData* data = Data(env);
  Builder* original = OwnerPointer<Builder>(owner);
  Builder* input = original;
  RichRenderable* output = nullptr;
  const RichResult result = std::forward<Finish>(finish)(&input, &output);

  const bool input_is_original = input == original;
  const bool input_is_replacement = input != nullptr && !input_is_original;
  const bool output_aliases_live_input =
      output != nullptr && input != nullptr && static_cast<void*>(input) == output;
  const bool success_shape = result == RICH_OK && input == nullptr && output != nullptr;
  const bool error_shape =
      result != RICH_OK && output == nullptr && (input == nullptr || input_is_original);
  const bool malformed = (!success_shape && !error_shape) || output_aliases_live_input;

  // The returned input value is the native side's ownership receipt. An unchanged
  // pointer remains caller-owned; NULL means consumed; a replacement is caller-owned
  // but violates this ABI and must be destroyed before reporting the mismatch.
  Builder* malformed_original = malformed && input_is_original ? original : nullptr;
  owner->pointer = input_is_original && !malformed ? original : nullptr;
  PendingOwner pending_original(data, kind, malformed_original);
  PendingOwner pending_replacement(data, kind, input_is_replacement ? input : nullptr);
  PendingOwner pending_output(data, OwnerKind::Renderable,
                              output_aliases_live_input ? nullptr : output);

  if (malformed) {
    ThrowRich(env, kAbiMismatch, operation,
              "native finish returned an inconsistent ownership state");
  }
  if (result != RICH_OK) {
    CheckResult(env, result, operation);
  }
  return pending_output.IntoObject(env, operation);
}

template <typename Builder, RichResult (*Finish)(Builder**, RichRenderable**)>
Napi::Value FinishOwner(const Napi::CallbackInfo& info, OwnerKind kind,
                        std::string_view operation) {
  Owner* owner = ThisOwner(info, kind, operation);
  return FinishOwnerChecked<Builder>(
      info.Env(), owner, kind, operation,
      [](Builder** input, RichRenderable** output) { return Finish(input, output); });
}

} // namespace rich_node

#endif
