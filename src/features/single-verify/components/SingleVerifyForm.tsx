import { useForm } from "@tanstack/react-form";
import { Loader2 } from "lucide-react";
import { Button } from "@/shared/components/ui/button";
import { Input } from "@/shared/components/ui/input";

export function SingleVerifyForm({
	loading,
	onSubmit,
	compact,
}: {
	loading: boolean;
	onSubmit: (email: string) => void;
	compact?: boolean;
}) {
	const form = useForm({
		defaultValues: { email: "" },
		onSubmit: async ({ value }) => {
			if (value.email.trim()) onSubmit(value.email.trim());
		},
	});

	return (
		<form
			className="flex gap-2"
			onSubmit={(e) => {
				e.preventDefault();
				e.stopPropagation();
				void form.handleSubmit();
			}}
		>
			<form.Field name="email">
				{(field) => (
					<Input
						className={compact ? "" : "text-base"}
						placeholder="输入要验证的邮箱地址"
						value={field.state.value}
						onChange={(e) => field.handleChange(e.target.value)}
						disabled={loading}
					/>
				)}
			</form.Field>
			<form.Subscribe selector={(state) => state.values.email}>
				{(email) => (
					<Button type="submit" disabled={loading || !email.trim()}>
						{loading ? (
							<>
								<Loader2 data-icon="inline-start" className="animate-spin" />
								验证中...
							</>
						) : (
							"验证"
						)}
					</Button>
				)}
			</form.Subscribe>
		</form>
	);
}
