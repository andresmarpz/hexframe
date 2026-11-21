import * as Schema from "effect/Schema";
import * as HttpApi from "effect/http-api/HttpApi";
import * as HttpApiEndpoint from "effect/http-api/HttpApiEndpoint";
import * as HttpApiGroup from "effect/http-api/HttpApiGroup";

export const maxMessageLength = 5000;
export const maxLogTailBytes = 200 * 1024;

const utf8Bytes = (text: string) => new TextEncoder().encode(text).length;

const LogTail = Schema.String.check(
  Schema.makeFilter((text: string) =>
    utf8Bytes(text) <= maxLogTailBytes ? undefined : `log_tail must be at most ${maxLogTailBytes} bytes`,
  ),
);

export const FeedbackKind = Schema.Literals(["bug", "idea", "other"]);

export const FeedbackPayload = Schema.Struct({
  kind: FeedbackKind,
  message: Schema.String.check(Schema.isMinLength(1), Schema.isMaxLength(maxMessageLength)),
  email: Schema.NullOr(Schema.String.check(Schema.isMaxLength(320))),
  app_version: Schema.String.check(Schema.isMinLength(1), Schema.isMaxLength(64)),
  os: Schema.String.check(Schema.isMinLength(1), Schema.isMaxLength(128)),
  log_tail: Schema.NullOr(LogTail),
  client_id: Schema.String.check(Schema.isUUID()),
});
export type FeedbackPayload = typeof FeedbackPayload.Type;

export const FeedbackAccepted = Schema.Struct({ id: Schema.String });

export const ApiErrorBody = Schema.Struct({ error: Schema.String });

export class BadRequest extends Schema.Error<BadRequest>("BadRequest")(
  { error: Schema.String },
  { httpApiStatus: 400 },
) {}

export class TooManyRequests extends Schema.Error<TooManyRequests>("TooManyRequests")(
  { error: Schema.String },
  { httpApiStatus: 429 },
) {}

export const HealthStatus = Schema.Struct({ ok: Schema.Literal(true), version: Schema.String });

const submitFeedback = HttpApiEndpoint.post("submitFeedback", "/v1/feedback", {
  payload: FeedbackPayload,
  success: FeedbackAccepted.annotate({ httpApiStatus: 202 }),
  error: [BadRequest, TooManyRequests],
});

const health = HttpApiEndpoint.get("health", "/health", {
  success: HealthStatus,
});

export class FeedbackGroup extends HttpApiGroup.make("feedback").add(submitFeedback) {}
export class SystemGroup extends HttpApiGroup.make("system").add(health) {}

export class GreyscreenApi extends HttpApi.make("GreyscreenApi").add(FeedbackGroup).add(SystemGroup) {}
