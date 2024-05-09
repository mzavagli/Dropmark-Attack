/* Copyright (c) 2001 Matej Pfajfar.
 * Copyright (c) 2001-2004, Roger Dingledine.
 * Copyright (c) 2004-2006, Roger Dingledine, Nick Mathewson.
 * Copyright (c) 2007-2024, The Tor Project, Inc. */
/* See LICENSE for licensing information */

/**
 * @file dropmark_attack.c
 * @brief DOCDOC
 **/

#include "orconfig.h"

#include "core/or/dropmark_attack.h"
#include "core/or/relay.h"
#include "core/or/circuit_st.h"
#include "core/or/or_circuit_st.h"
#include "core/or/circuitlist.h"
#include "core/or/channeltls.h"
#include "core/or/channel.h"
#include "core/or/or_connection_st.h"

#include "feature/nodelist/node_st.h"
#include "feature/nodelist/nodelist.h"
#include "feature/relay/router.h"

#include "app/config/config.h"

#include "lib/crypt_ops/crypto_rand.h"
#include "src/lib/evloop/compat_libevent.h"

static int 
signal_send_relay_drop(int nbr, circuit_t *circ) 
{
  int random_streamid = 0;
  if (get_options()->FakeDataCell) {
    random_streamid = crypto_rand_int(65536);
  }
  while (nbr > 0) {
    if (get_options()->FakeDataCell) {
      if (relay_send_command_from_edge_(random_streamid, circ,
            RELAY_COMMAND_DATA, NULL, 0,
            NULL, __FILE__, __LINE__) < 0) {
        log_debug(LD_BUG, "DROPMARK: Signal not completly sent");
        return -1;
      }
    }
    else {
      if (relay_send_command_from_edge_(0, circ,
                                  RELAY_COMMAND_DROP, NULL, 0,
                                  NULL, __FILE__, __LINE__) < 0) {
        log_debug(LD_BUG, "DROPMARK: Signal not completly sent");
        return -1;
      }
    }
    nbr--;
  }

  return 0;
}

// -------------------------------| Injecting |-----------------------------------

static void 
signal_encode_simple_watermark(circuit_t *circ) {

  
  if (signal_send_relay_drop(3, circ) < 0) {
    log_info(LD_GENERAL, "DROPMARK: signal_send_relay_drop returned -1 when sending the watermark");
  }
  if (!CIRCUIT_IS_ORIGIN(circ)) {
    channel_flush_some_cells(TO_OR_CIRCUIT(circ)->p_chan, -1);
    connection_flush(TO_CONN(BASE_CHAN_TO_TLS(TO_OR_CIRCUIT(circ)->p_chan)->conn));
  }
}

void 
signal_encode_destination(void *p) 
{
  struct signal_encode_param_t *param = p;
  char *address = param->address;
  circuit_t *circ = param->circ;
  const or_options_t *options = get_options();
  switch (options->SignalMethod) {
    case SIMPLE_WATERMARK: signal_encode_simple_watermark(circ);
            break;
  }
}

// -------------------------------| Decoding |-----------------------------------

static smartlist_t *circ_timings;

static int
signal_compare_signal_decode_(const void **a_, const void **b_) {
    const dropmark_decode_t *a = *a_;
    const dropmark_decode_t *b = *b_;
    circid_t circid_a = a->circid;
    circid_t circid_b = b->circid;
    
    if (circid_a < circid_b) {return -1;}
    else if (circid_a == circid_b) {return 0;}
    else {return 1;}
}

static int
signal_compare_key_to_entry_(const void *_key, const void **_member) {
    const circid_t circid = *(circid_t*)_key;
    const dropmark_decode_t *entry = *_member;
    
    if (circid < entry->circid) {return -1;}
    else if (circid == entry->circid) {return 0;}
    else {return 1;}
}

static void 
handle_timing_add(dropmark_decode_t *circ_timing, struct timespec *now, int SignalMethod) 
{
  smartlist_add(circ_timing->timespec_list, now);
  circ_timing->last = *now;
  switch (SignalMethod) {
    case SIMPLE_WATERMARK:
      if (smartlist_len(circ_timing->timespec_list) > 5) {
        tor_free(circ_timing->timespec_list->list[0]);
        smartlist_del_keeporder(circ_timing->timespec_list, 0);
        circ_timing->first = *(struct timespec *) smartlist_get(circ_timing->timespec_list,0);
      }
      break;
    default:
      log_info(LD_GENERAL, "DROPMARK: handle_timing_add default case reached. It should not happen");
  }
}

static int 
delta_timing(struct timespec *t1, struct timespec *t2) 
{
  const or_options_t *options = get_options();
  double elapsed_ms = (t2->tv_sec-t1->tv_sec)*1000.0 +\
                      (t2->tv_nsec-t1->tv_nsec)*1E-6;
  log_info(LD_GENERAL, "elapsed = %f", elapsed_ms);

  if (elapsed_ms >= (options->SignalBlankIntervalMS*0.95))
    return 0;
  else if (elapsed_ms >= 0)
    return 1;
  else {
    log_info(LD_GENERAL, "DROPMARK: BUG: delta_timing compute a negative delta");
    return -1;
  }
}

static int
signal_decode_simple_watermark(dropmark_decode_t *circ_timing, char *p_addr, char *n_addr) {
    if (smartlist_len(circ_timing->timespec_list) == 5) {
        int count = 0;
        
        if (delta_timing(smartlist_get(circ_timing->timespec_list, 0), smartlist_get(circ_timing->timespec_list, 1)) == 0) {count++;}
        if (delta_timing(smartlist_get(circ_timing->timespec_list, 1), smartlist_get(circ_timing->timespec_list, 2)) == 1) {count++;}
        if (delta_timing(smartlist_get(circ_timing->timespec_list, 2), smartlist_get(circ_timing->timespec_list, 3)) == 1) {count++;}
        if (delta_timing(smartlist_get(circ_timing->timespec_list, 3), smartlist_get(circ_timing->timespec_list, 4)) == 0) {count++;}
        
        if (count == 4) {
          log_info(LD_GENERAL, "DROPMARK: Spotted watermark, predecessor: %s, successor: %s", p_addr, n_addr);
          smartlist_clear(circ_timing->timespec_list);
          }
        else {log_info(LD_GENERAL, "DROPMARK: No watermark count:%d, predecessor: %s, successor: %s", count, p_addr, n_addr);}

        return 1;
    }
    else {
        (void) p_addr;
        (void) n_addr;
        return 0;
    }
}

static circid_t counter = 1;

int signal_listen_and_decode(circuit_t *circ) {
  or_circuit_t *or_circ = NULL;
  if (!circ_timings)
    circ_timings = smartlist_new();
  const or_options_t *options = get_options();
  dropmark_decode_t *circ_timing;
  circid_t circid = circ->timing_circ_id;
  struct timespec *now = tor_malloc_zero(sizeof(struct timespec));
  circ_timing = smartlist_bsearch(circ_timings, &circid, 
      signal_compare_key_to_entry_);
  if (!CIRCUIT_IS_ORIGIN(circ))
    or_circ = TO_OR_CIRCUIT(circ);
  tor_addr_t p_tmp_addr, n_tmp_addr;
  char p_addr[TOR_ADDR_BUF_LEN], n_addr[TOR_ADDR_BUF_LEN];
  if (channel_get_addr_if_possible(or_circ->p_chan, &p_tmp_addr)) {
    tor_addr_to_str(p_addr, &p_tmp_addr, TOR_ADDR_BUF_LEN, 0);
  }
  else
    p_addr[0] = '\0';

  clock_gettime(CLOCK_REALTIME, now);
  if (!circ_timing) {
    circ_timing = tor_malloc_zero(sizeof(dropmark_decode_t));
    circ->timing_circ_id = counter;
    circ_timing->circid = counter++;
    circ_timing->timespec_list = smartlist_new();
    circ_timing->first = *now;
    smartlist_insert_keeporder(circ_timings, circ_timing,
        signal_compare_signal_decode_);
    SMARTLIST_FOREACH(nodelist_get_list(), node_t *, node,
    {
      if (node->ri) {
        if (router_has_addr(node->ri, &p_tmp_addr)) {
          circ_timing->disabled = 1;
        }
      }
    });
  }
  if (circ_timing->disabled){
    tor_free(now);
    return 1;
  }

  circ_timing->last = *now;
  if (channel_get_addr_if_possible(circ->n_chan, &n_tmp_addr)) {
    tor_addr_to_str(n_addr, &n_tmp_addr, TOR_ADDR_BUF_LEN, 0);
  }
  else
    n_addr[0] = '\0';
  
  handle_timing_add(circ_timing, now, options->SignalMethod);
  switch (options->SignalMethod)
  {
  case SIMPLE_WATERMARK:
    return signal_decode_simple_watermark(circ_timing, p_addr, n_addr);
  
  default:
    break;
  }

  return -1;
}

// -------------------------------| Cleaning |-----------------------------------

void 
signal_free(circuit_t *circ) {
  if (!circ_timings)
    return;
  circid_t circid = circ->n_circ_id;
  int found;
  int idx = smartlist_bsearch_idx(circ_timings, &circid, signal_compare_key_to_entry_, &found);
  if (found) {
    smartlist_free(((dropmark_decode_t *)smartlist_get(circ_timings, idx))->timespec_list);
    tor_free(circ_timings->list[idx]);
    smartlist_del_keeporder(circ_timings, idx);
  }
}