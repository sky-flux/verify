import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { z } from "zod";

const appErrorSchema = z.object({
	message: z.string(),
	field: z.string().nullable(),
});

export class AppCommandError extends Error {
	/** The Settings field this error belongs to, e.g. "maxConcurrentDomains" —
	 * present only for AppError::InvalidSetting, null for every other Rust
	 * error variant (see src-tauri/src/error.rs's Serialize impl). */
	field: string | null;

	constructor(message: string, field: string | null = null) {
		super(message);
		this.name = "AppCommandError";
		this.field = field;
	}
}

/**
 * Thin wrapper around Tauri's `invoke` so every command call goes through
 * one place: Rust commands return `Result<T, AppError>`, and AppError
 * serializes as `{ message, field }` (see src-tauri/src/error.rs), so a
 * rejected promise here always carries a human-readable Chinese message
 * straight from the backend — plus, for settings validation errors, which
 * field it belongs to — nothing to parse or re-derive on the frontend.
 *
 * `invoke<T>()`'s generic is a compile-time-only assertion — it does not
 * check anything at runtime. Passing a zod `schema` here parses the actual
 * response, so a drift between the Rust struct and the TS type (a renamed
 * field, a dropped serde rename) throws immediately at the call site
 * instead of silently propagating wrong data into the UI.
 */
export async function callCommand<S extends z.ZodType>(
	command: string,
	schema: S,
	args?: Record<string, unknown>,
): Promise<z.infer<S>> {
	let raw: unknown;
	try {
		raw = await invoke(command, args);
	} catch (error) {
		const parsedError = appErrorSchema.safeParse(error);
		if (parsedError.success) {
			throw new AppCommandError(
				parsedError.data.message,
				parsedError.data.field,
			);
		}
		throw new AppCommandError(
			typeof error === "string" ? error : String(error),
		);
	}
	const parsed = schema.safeParse(raw);
	if (!parsed.success) {
		throw new AppCommandError(
			`${command} 返回了意料之外的数据结构: ${parsed.error.message}`,
		);
	}
	return parsed.data;
}

export function onEvent<S extends z.ZodType>(
	event: string,
	schema: S,
	handler: (payload: z.infer<S>) => void,
): Promise<UnlistenFn> {
	return listen<unknown>(event, (e) => {
		const parsed = schema.safeParse(e.payload);
		if (parsed.success) {
			handler(parsed.data);
		}
	});
}
