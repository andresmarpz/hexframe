import * as Effect from "effect/Effect";
import * as HttpRouter from "effect/http/HttpRouter";
import * as HttpServerResponse from "effect/http/HttpServerResponse";
import { HttpApiPayloadTooLarge, HttpApiSchemaError } from "effect/http-api/HttpApiError";

const errorResponse = (status: number, error: string) =>
  HttpServerResponse.jsonUnsafe({ error }, { status });

const summarize = (error: HttpApiSchemaError) =>
  `invalid ${error.kind.toLowerCase()}: ${error.cause.message}`.slice(0, 500);

const isBodyTooLarge = (defect: unknown) => {
  const error = defect as { _tag?: string; reason?: { _tag?: string; reason?: string } } | null;
  return (
    error?._tag === "HttpServerError" &&
    error.reason?._tag === "RequestParseError" &&
    error.reason.reason === "BodyTooLarge"
  );
};

const emptyClientErrorMessages: Record<number, string> = {
  400: "bad request",
  404: "not found",
  405: "method not allowed",
  413: "request body too large",
};

const fillEmptyError = (response: HttpServerResponse.HttpServerResponse) => {
  const message = emptyClientErrorMessages[response.status];
  return message !== undefined && response.body._tag === "Empty" ? errorResponse(response.status, message) : response;
};

export const JsonErrors = HttpRouter.middleware(
  (app) =>
    app.pipe(
      Effect.catchDefect((defect) => {
        if (HttpApiSchemaError.is(defect) && defect.kind !== "ResponseHeaders") {
          return Effect.succeed(errorResponse(400, summarize(defect)));
        }
        if (HttpApiPayloadTooLarge.is(defect) || isBodyTooLarge(defect)) {
          return Effect.succeed(errorResponse(413, "request body too large"));
        }
        return Effect.die(defect);
      }),
      Effect.map(fillEmptyError),
    ),
  { global: true },
);
