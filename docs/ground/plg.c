#define _ZS_PLUGIN__
#define _GNU_SOURCE
#include "plg.h"
#include "string/xcpy.h"
#include <dlfcn.h>
#include <stdlib.h>
#include <string.h>

static void zsplg_setstdl(zsplg_handle_t *const handle, const zsplg_status st) {
  handle->st        = st;
  handle->error_str = dlerror();
}

zsplg_handle_t zsplg_open(const char * restrict file, const char * restrict modname, const bool do_strcpy) {
  /* load plugin with dlopen */
  zsplg_handle_t ret = {
    .st         = ZSP_OK,
    .modname    = 0,
    .mnlen      = strlen(modname),
    .dlh        = file ? dlopen(file, RTLD_LAZY | RTLD_LOCAL) : RTLD_DEFAULT,
    .have_alloc = do_strcpy,
  };
  if(zs_unlikely(file && !ret.dlh)) {
    zsplg_setstdl(&ret, ZSPE_DLOPN);
    return ret;
  }

  /* init_fn_name = "init_" + modname + "\0" */
  zsplugin_t* (*init_fn)() = 0;
  {
    char init_fn_name[ret.mnlen + 6];
    char *tmp = init_fn_name;
    llzs_strixcpy(&tmp, "init_", 5);
    llzs_strixcpy(&tmp, modname, ret.mnlen);
    init_fn = dlsym(ret.dlh, init_fn_name);
  }

  /* initialize plugin */
  if(zs_likely(init_fn)) {
    ret.plugin  = *init_fn();
    ret.modname = do_strcpy ? llzs_strxdup(modname, ret.mnlen) : modname;
  } else {
    zsplg_setstdl(&ret, ZSPE_DLOPN);
    /* cleanup dlh */
    if(file) {
      dlclose(ret.dlh);
      ret.dlh = 0;
    }
  }
  return ret;
}

bool zsplg_destroy(zsplg_gdsa_t *const gdsa) {
  // gdsa->destroy != 0 --> successful
  const bool ret = gdsa && (!gdsa->destroy || gdsa->destroy(gdsa->data));
  if(ret) {
    gdsa->destroy = 0;
    gdsa->data = 0;
    gdsa->len = 0;
  }
  return ret;
}

bool zsplg_close(zsplg_handle_t *const handle) {
  if(zs_unlikely(!handle))
    return false;

  zsplg_status st = handle->st;
  if(st == ZSPE_DLOPN)
    return false;

  zsplugin_t *const plgptr = &handle->plugin;
  if(zs_unlikely(!zsplg_destroy(&plgptr->data)))
    st = ZSPE_PLG;

  /* unload plugin */
  if(!handle->dlh) {
    /* nothing to unload */
  } else if(zs_likely(!dlclose(handle->dlh))) {
    handle->dlh = 0;
  } else {
    if(st == ZSP_OK) st = ZSPE_DLCLOS;
    handle->error_str = dlerror();
  }

  if(handle->have_alloc) {
    free((void*)handle->modname);
    handle->modname = 0;
  }

  return ZSP_OK == (handle->st = st);
}

zsplg_gdsa_t zsplg_h_create(const zsplg_handle_t *const base, size_t argc, const char *argv[]) {
  const zsplugin_t *const plgptr = &base->plugin;
  return plgptr->fn_h_create(plgptr->data.data, argc, argv);
}

static void zsplg_upd_errstr(zsplg_handle_t *const handle, const zsplg_status st) {
  const char *const tmp = dlerror();
  if(zs_likely(!tmp)) return;
  handle->error_str = tmp;
  if(st != ZSP_OK) handle->st = st;
}

zs_attrib(hot)
zsplg_gdsa_t zsplg_call_h(zsplg_handle_t *const handle, void *const h_id, const char *fn, size_t argc, const char *argv[]) {
  /* handle error conditions */
  if(zs_unlikely(!handle || !fn || handle->st == ZSPE_DLOPN))
    RET_GDSA_NULL;

  zsplg_gdsa_t (*xfn_ptr)(void *, size_t, const char *const*);

  {
    /* construct function name */
    const size_t fnlen = strlen(fn);
    char xfn_name[handle->mnlen + fnlen + 3 + (h_id ? 1 : 0)];
    {
      char *xnp = llzs_strxcpy(xfn_name, handle->modname, handle->mnlen);
               *(xnp++) = '_';
      if(h_id) *(xnp++) = 'h';
      llzs_strxcpy(stpcpy(xnp, "_"), fn, fnlen);
    }

    /* get function addr */
    zsplg_upd_errstr(handle, ZSP_OK);
    xfn_ptr = dlsym(handle->dlh, xfn_name);
  }

  if(zs_unlikely(!xfn_ptr)) {
    zsplg_upd_errstr(handle, ZSPE_DLSYM);
    RET_GDSA_NULL;
  }

  /* call function */
  return xfn_ptr(h_id ? h_id : handle->plugin.data.data, argc, argv);
}
