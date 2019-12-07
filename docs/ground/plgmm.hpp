/** plgmm.hpp
 * (C) 2018 Erik Zscheile
 */
#pragma once
#include "plg.h"
#include <memory>

namespace llzs {
namespace zsplg {
  struct opaque { };

  class gdsa_helper_t {
   public:
    typedef bool (*destroyer_t)(void*);
   private:
    const destroyer_t _destroyer;
   public:
    gdsa_helper_t(const destroyer_t destroyer)
      : _destroyer(destroyer) { }
    void operator()(void *const ptr) const noexcept;
  };

  class gdsa final {
    std::shared_ptr<opaque> _ptr;
    size_t _len;
   public:
    gdsa(): _len(0) { }
    // the constructor transfers ownership from the argument into the class
    gdsa(const zsplg_gdsa_t &gdsa);
    size_t size() const noexcept { return _len; }
    void*  get() noexcept { return _ptr.get(); }
    auto   get() const noexcept -> const void * { return _ptr.get(); }
    auto begin() const noexcept -> const void * { return _ptr.get(); }
    auto   end() const noexcept -> const void * { return _ptr.get() + _len; }
  };

  struct callable_gdsa_base {
    virtual ~callable_gdsa_base() = default;
    gdsa operator()(const char * __restrict__ fn, ...);
    virtual gdsa call_argv(const char * __restrict__ fn, size_t argc, const char *argv[]) = 0;
  };

  class plugin final : public callable_gdsa_base {
    zsplg_handle_t _plg;
   public:
    plugin(const char * __restrict__ file, const char * __restrict__ modname, bool do_strcpy);
    plugin(const plugin &o) = delete;
    ~plugin();
    auto get() noexcept -> zsplg_handle_t*
      { return &_plg; }
    zsplg_status status() const noexcept
      { return _plg.st; }
    auto error_str() const noexcept -> const char *
      { return _plg.error_str; }
    gdsa call_argv(const char * __restrict__ fn, size_t argc, const char *argv[]);
  };

  class handle final : public callable_gdsa_base {
    plugin &_plg;
    gdsa _hdl;
   public:
    handle(plugin &plg, const char *sub);
    handle(plugin &plg, size_t argc, const char *argv[]);
    gdsa call_argv(const char * __restrict__ fn, size_t argc, const char *argv[]);
  };
}
}
