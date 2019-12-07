/** dtors4plugins.h
 * (C) 2018 Erik Zscheile
 */
#pragma once
#include <stdbool.h>
#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif
  bool _Z10do_destroyP8_IO_FILE(FILE *f); /* fclose */
  bool _Z10do_destroyPv(void *ptr);       /* free */
#ifdef __cplusplus
}
#endif
