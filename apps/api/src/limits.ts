export const rateLimit = { perHour: 10, windowMs: 60 * 60 * 1000 } as const;
export const maxFeedbackBodyKib = 512;
export const maxCrashBodyBytes = 25 * 1024 * 1024;
export const maxMinidumpBytes = 20 * 1024 * 1024;
export const maxCrashMetaBytes = 64 * 1024;
