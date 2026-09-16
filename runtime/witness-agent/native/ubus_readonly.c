#include <stdlib.h>
#include <string.h>
#include <libubus.h>
#include <libubox/blobmsg_json.h>

#define INTENT_UBUS_RESPONSE_TOO_LARGE 7001
#define INTENT_UBUS_TIMEOUT 7002
#define INTENT_UBUS_DENIED 7003
#define INTENT_UBUS_MALFORMED 7004

static int allowed(const char *object, const char *method) {
  return (!strcmp(object, "session") && !strcmp(method, "access")) ||
    (!strcmp(object, "uci") && (!strcmp(method, "get") || !strcmp(method, "changes"))) ||
    (!strcmp(object, "network.interface") && !strcmp(method, "dump")) ||
    (!strcmp(object, "network.device") && !strcmp(method, "status")) ||
    (!strcmp(object, "network.wireless") && !strcmp(method, "status"));
}
struct response { char *json; size_t max; int error; unsigned int messages; };
static void receive(struct ubus_request *req, int type, struct blob_attr *msg) {
  struct response *response = req->priv;
  char *json;
  (void)type;
  if (response->error) return;
  /* Each supported read returns exactly one JSON table. Never silently keep
   * only the final fragment of a multipart response and label it complete. */
  if (++response->messages != 1 || !msg) {
    response->error = INTENT_UBUS_MALFORMED;
    return;
  }
  /* Bound the input before JSON formatting allocates. The formatted output
   * is checked separately since escaping may enlarge it. */
  if (blob_len(msg) > response->max) {
    response->error = INTENT_UBUS_RESPONSE_TOO_LARGE;
    return;
  }
  json = blobmsg_format_json(msg, true);
  if (!json) { response->error = UBUS_STATUS_UNKNOWN_ERROR; return; }
  if (strlen(json) > response->max) { free(json); response->error = INTENT_UBUS_RESPONSE_TOO_LARGE; return; }
  free(response->json); response->json = json;
}
int intent_ubus_invoke_json(const char *object, const char *method, const char *request,
  int timeout_ms, size_t max_response_bytes, char **out) {
  struct ubus_context *ctx; struct blob_buf buffer = {}; uint32_t id; int status;
  struct response response = { .json = NULL, .max = max_response_bytes, .error = 0, .messages = 0 };
  if (!out || !object || !method || !request || !allowed(object, method) || timeout_ms <= 0 || max_response_bytes == 0) return UBUS_STATUS_INVALID_ARGUMENT;
  *out = NULL; blob_buf_init(&buffer, 0); ctx = ubus_connect(NULL); if (!ctx) { blob_buf_free(&buffer); return UBUS_STATUS_CONNECTION_FAILED; }
  if (!blobmsg_add_json_from_string(&buffer, request)) { status = UBUS_STATUS_INVALID_ARGUMENT; goto done; }
  status = ubus_lookup_id(ctx, object, &id); if (status) goto done;
  status = ubus_invoke(ctx, id, method, buffer.head, receive, &response, timeout_ms);
  if (!status && response.error) status = response.error;
  if (!status && !response.json) status = UBUS_STATUS_NO_DATA;
  if (!status) { *out = response.json; response.json = NULL; }
done:
  free(response.json); blob_buf_free(&buffer); ubus_free(ctx);
  if (status == UBUS_STATUS_PERMISSION_DENIED) return INTENT_UBUS_DENIED;
  if (status == UBUS_STATUS_TIMEOUT) return INTENT_UBUS_TIMEOUT;
  return status;
}
void intent_ubus_free(char *value) { free(value); }
