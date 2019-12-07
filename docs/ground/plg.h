/** plg.h
 * (C) 2018 Erik Zscheile

  zsplg functions
    zsplg_open : open's a plugin + init of plugin
      cleanup with zsplg_close even in case of error in zsplg_status
    zsplg_close: deallocates internal vars, deinit plugin, close's a plugin

 */
#pragma once
#include "llzs_config.h"
#include "plg4plugins.h"
#include <stdbool.h>
#include <stddef.h>

typedef enum {
  ZSP_OK, ZSPE_DLOPN, ZSPE_DLCLOS,
  ZSPE_DLSYM, ZSPE_PLG
} zsplg_status;

typedef struct {
  zsplugin_t plugin;
  zsplg_status st;
  size_t mnlen;
  const char *modname, *error_str;
  void *dlh;
  bool have_alloc;
} zsplg_handle_t;

#ifdef __cplusplus
extern "C" {
#endif
  zsplg_handle_t zsplg_open(const char * __restrict__ file, const char * __restrict__ modname, bool do_strcpy);
  bool zsplg_close(zsplg_handle_t *handle);
  bool zsplg_destroy(zsplg_gdsa_t *gdsa);

  zsplg_gdsa_t zsplg_h_create(const zsplg_handle_t *base, size_t argc, const char *argv[]);
  zsplg_gdsa_t zsplg_call_h(zsplg_handle_t *const base, void *const h_id, const char *fn, size_t argc, const char *argv[]);
#ifdef __cplusplus
}
#endif
#define zsplg_gdsa_get(GDSA) ((GDSA).data)
#define zsplg_h_call(BASE,HANDLE,FN,ARGC,ARGV) zsplg_call_h(BASE, (HANDLE).data, FN, ARGC, ARGV)
