/** plgmm.cxx
 * (C) 2018 Erik Zscheile
 */
#include "plgmm.hpp"
#include <stdarg.h>
#include <vector>

using namespace std;

namespace llzs {
namespace zsplg {

void gdsa_helper_t::operator()(void *const ptr) const noexcept {
  if(_destroyer) _destroyer(ptr);
}

gdsa::gdsa(const zsplg_gdsa_t &gdsa)
  : _ptr(reinterpret_cast<opaque*>(gdsa.data), gdsa_helper_t(gdsa.destroy)),
    _len(gdsa.len) { }

gdsa callable_gdsa_base::operator()(const char * __restrict__ fn, ...) {
  vector<const char *> args;
  va_list ap;
  va_start(ap, fn);
  while(true) {
    char * arg = va_arg(ap, char *);
    if(!arg) break;
    args.emplace_back(arg);
  }
  return call_argv(fn, args.size(), args.data());
}

plugin::plugin(const char * __restrict__ file, const char * __restrict__ modname, bool do_strcpy)
  : _plg(zsplg_open(file, modname, do_strcpy)) { }

plugin::~plugin()
  { zsplg_close(&_plg); }

gdsa plugin::call_argv(const char * __restrict__ fn, size_t argc, const char *argv[]) {
  return zsplg_call_h(&_plg, 0, fn, argc, argv);
}

handle::handle(plugin &plg, size_t argc, const char *argv[])
  : _plg(plg) {
  _hdl = zsplg_h_create(plg.get(), argc, argv);
}

handle::handle(plugin &plg, const char *sub)
  : _plg(plg) {
  const char *argv[] = { sub, 0 };
  _hdl = zsplg_h_create(plg.get(), 1, argv);
}

gdsa handle::call_argv(const char * __restrict__ fn, size_t argc, const char *argv[]) {
  return zsplg_call_h(_plg.get(), _hdl.get(), fn, argc, argv);
}

}
}
