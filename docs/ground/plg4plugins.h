/** plg4plugins.h
 * (C) 2018 Erik Zscheile

  expected plugin functions
    zsplugin_t * MODNAME_init()
    void * MODNAME__XFN(void * data, size_t argc, char *argv[])
    void * MODNAME_h_XFN(void * handle, size_t argc, char *argv[])

 */
#pragma once
#include <stdbool.h>
#include <stddef.h>

typedef struct zsplg_gdsa {
  /* destroy invoc: .destroy(.data) */
  void* data;
  size_t len;
  bool (*destroy)(void *data);
} zsplg_gdsa_t;

typedef struct {
  zsplg_gdsa_t data;
  zsplg_gdsa_t (*fn_h_create)(void *data, size_t argc, const char *argv[]);
} zsplugin_t;

#ifdef _ZS_PLUGIN__
# define ZS_GDSA(X,Y,Z) ((zsplg_gdsa_t) { (X), (Y), (bool(*)(void*))(Z) })
# define ZS_GDSA_NULL ZS_GDSA(0, 0, 0)
# define RET_GDSA(X,Y,Z) return ZS_GDSA((X), (Y), (Z))
# define RET_GDSA_NULL return ZS_GDSA_NULL
#endif
