/* Copyright (c) 2001 Matej Pfajfar.
 * Copyright (c) 2001-2004, Roger Dingledine.
 * Copyright (c) 2004-2006, Roger Dingledine, Nick Mathewson.
 * Copyright (c) 2007-2024, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * @file dropmark_attack.h
 * @brief Header for core/or/dropmark_attack.c
 **/

#ifndef TOR_CORE_OR_DROPMARK_ATTACK_H
#define TOR_CORE_OR_DROPMARK_ATTACK_H

#define BANDWIDTH_EFFICIENT 0
#define MIN_BLANK 1
#define SIMPLE_WATERMARK 2
#define SIGNAL_ATTACK_MAX_BLANK 2000

#include "core/or/or.h"

#include <stdlib.h>
#include <event2/event.h>

typedef struct dropmark_decode_t {
    circid_t circid;
    struct timespec first;
    struct timespec last;
    smartlist_t *timespec_list;
    int disabled;
} dropmark_decode_t;

typedef struct signal_encode_param_t {
  char *address;
  circuit_t *circ;
} signal_encode_param_t;

typedef struct dropmark_encode_state_t {
  int nb_calls;
  circuit_t *circ;
  int subip[4];
  char *address;
  struct event *ev;
} dropmark_encode_state_t;


//void signal_encode_destination(char *address, circuit_t *circ);
void signal_encode_destination(void *param);

int signal_listen_and_decode(circuit_t *circ);

void signal_free(circuit_t *circ);

#ifdef TOR_SIGNALATTACK_PRIVATE
static int signal_compare_signal_decode_(const void **a_, const void **b_);
static int signal_compare_key_to_entry_(const void *_key, const void **_member);
static void signal_encode_simple_watermark(circuit_t *circuit);
#endif

#endif /* !defined(TOR_CORE_OR_DROPMARK_ATTACK_H) */
