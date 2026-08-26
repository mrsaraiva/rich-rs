#include <node_api.h>

#include <cstdint>

namespace {

struct ForeignOwner {
  std::uint64_t marker{UINT64_C(0x464f524549474e21)};
};

void Finalize(napi_env, void* data, void*) { delete static_cast<ForeignOwner*>(data); }

napi_value MakeForeign(napi_env env, napi_callback_info) {
  napi_value object = nullptr;
  if (napi_create_object(env, &object) != napi_ok) {
    return nullptr;
  }
  auto* owner = new ForeignOwner();
  if (napi_wrap(env, object, owner, Finalize, nullptr, nullptr) != napi_ok) {
    delete owner;
    return nullptr;
  }
  return object;
}

napi_value Init(napi_env env, napi_value exports) {
  napi_value make = nullptr;
  if (napi_create_function(env, "makeForeign", NAPI_AUTO_LENGTH, MakeForeign, nullptr, &make) !=
      napi_ok) {
    return nullptr;
  }
  if (napi_set_named_property(env, exports, "makeForeign", make) != napi_ok) {
    return nullptr;
  }
  return exports;
}

} // namespace

NAPI_MODULE(NODE_GYP_MODULE_NAME, Init)
