/*
 * Runtime smoke test for runtime/witness-agent/native/ubus_readonly.c.
 *
 * This deliberately declares the narrow exported ABI instead of including
 * private shim state.  It exercises a permitted read and proves that a write
 * method cannot be sent through the shim.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int intent_ubus_invoke_json(const char *object, const char *method,
                            const char *request, int timeout_ms,
                            size_t max_response_bytes, char **out);
void intent_ubus_free(char *value);

static int invoke(const char *object, const char *method, const char *request) {
  char *response = NULL;
  int status = intent_ubus_invoke_json(object, method, request, 3000, 262144,
                                       &response);
  if (status != 0) {
    fprintf(stderr, "ubus %s.%s failed: %d\n", object, method, status);
    return status;
  }
  puts(response);
  intent_ubus_free(response);
  return 0;
}

static int self_test(void) {
  /* A write name is outside the C allowlist, so it must fail before RPC. */
  char *response = NULL;
  int denied = intent_ubus_invoke_json("uci", "set",
      "{\"config\":\"network\",\"section\":\"lan\"}",
      3000, 262144, &response);
  if (denied == 0 || response != NULL) {
    fprintf(stderr, "write method unexpectedly accepted (status=%d)\n", denied);
    if (response != NULL) intent_ubus_free(response);
    return 1;
  }
  fprintf(stderr, "write denial verified (uci.set status=%d)\n", denied);
  if (invoke("uci", "get", "{\"config\":\"network\"}") != 0) return 1;

  /* The same live response cannot fit in one byte. The shim must reject it
   * before JSON formatting can allocate an unbounded string. */
  response = NULL;
  denied = intent_ubus_invoke_json("uci", "get", "{\"config\":\"network\"}",
                                   3000, 1, &response);
  if (denied != 7001 || response != NULL) {
    fprintf(stderr, "small response budget was not rejected (status=%d)\n", denied);
    if (response != NULL) intent_ubus_free(response);
    return 1;
  }
  fprintf(stderr, "response-budget denial verified (uci.get status=%d)\n", denied);
  return 0;
}

int main(int argc, char **argv) {
  if (argc == 2 && strcmp(argv[1], "--self-test") == 0) return self_test() == 0 ? 0 : 1;
  if (argc != 4) {
    fprintf(stderr, "usage: %s --self-test | OBJECT METHOD JSON\n", argv[0]);
    return 64;
  }
  return invoke(argv[1], argv[2], argv[3]) == 0 ? 0 : 1;
}
