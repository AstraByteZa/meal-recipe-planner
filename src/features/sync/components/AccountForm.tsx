import { useState } from "react";
import { useForm } from "@tanstack/react-form";
import { Alert, Anchor, Button, Stack, TextInput } from "@mantine/core";
import { z } from "zod";
import { useSyncLogin, useSyncRegister } from "../mutations";
import { accountFormSchema } from "../schema";
import { fieldError } from "./formErrors";
import classes from "./AccountForm.module.css";

interface AccountFormProps {
	/** Pre-fills the server field. Usually the status' server URL or the build default. */
	initialServerUrl: string;
	/** Hides the server field behind "Advanced" when a valid default exists. */
	advancedServer?: boolean;
	onSignedIn?: () => void;
}

const formSchema = accountFormSchema.extend({ displayName: z.string() });

const isValidServerUrl = (value: string) => accountFormSchema.shape.serverUrl.safeParse(value).success;

/**
 * Sign in or register. Used on the sign-in page and in the sync settings modal, so both behave the
 * same way. Register adds an optional display name.
 */
export function AccountForm({ initialServerUrl, advancedServer = false, onSignedIn }: AccountFormProps) {
	const [mode, setMode] = useState<"signIn" | "register">("signIn");
	const [advancedOpen, setAdvancedOpen] = useState(false);
	const [showPassword, setShowPassword] = useState(false);
	const login = useSyncLogin();
	const register = useSyncRegister();
	const attempt = mode === "register" ? register : login;

	// Show the server field if there is no usable default, so the user can fix it.
	const serverVisible = !advancedServer || advancedOpen || !isValidServerUrl(initialServerUrl);

	const form = useForm({
		defaultValues: { serverUrl: initialServerUrl, displayName: "", email: "", password: "" },
		validators: { onChange: formSchema },
		onSubmit: ({ value }) => {
			const account = {
				serverUrl: value.serverUrl.trim(),
				email: value.email.trim(),
				password: value.password,
			};
			const done = { onSuccess: () => { form.setFieldValue("password", ""); onSignedIn?.(); } };
			if (mode === "register") {
				const displayName = value.displayName.trim();
				register.mutate({ ...account, displayName: displayName || undefined }, done);
			} else {
				login.mutate(account, done);
			}
		},
	});

	return <form onSubmit={(event) => {
		event.preventDefault();
		form.handleSubmit();
	}}>
		<Stack gap="sm">
			{serverVisible
				? <form.Field name="serverUrl">
					{(field) => <TextInput
						label="Server address"
						placeholder="https://sync.example.com"
						value={field.state.value}
						onChange={(event) => field.handleChange(event.currentTarget.value)}
						onBlur={field.handleBlur}
						error={fieldError(field)}
					/>}
				</form.Field>
				: <Button variant="subtle" color="gray" size="compact-sm" className={classes.advanced} onClick={() => setAdvancedOpen(true)}>
					Advanced: server address
				</Button>}

			{mode === "register" && <form.Field name="displayName">
				{(field) => <TextInput
					label="Your name"
					description="Optional. Shown to your family instead of your email."
					autoComplete="name"
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
				/>}
			</form.Field>}

			<form.Field name="email">
				{(field) => <TextInput
					label="Email"
					type="email"
					placeholder="you@example.com"
					autoComplete="email"
					value={field.state.value}
					onChange={(event) => field.handleChange(event.currentTarget.value)}
					onBlur={field.handleBlur}
					error={fieldError(field)}
				/>}
			</form.Field>

			<form.Field name="password">
				{(field) => <div className={classes.passwordRow}>
					<TextInput
						label="Password"
						type={showPassword ? "text" : "password"}
						autoComplete={mode === "register" ? "new-password" : "current-password"}
						value={field.state.value}
						onChange={(event) => field.handleChange(event.currentTarget.value)}
						onBlur={field.handleBlur}
						error={fieldError(field)}
						className={classes.passwordInput}
					/>
					<Button
						variant="default"
						className={classes.showButton}
						aria-pressed={showPassword}
						onClick={() => setShowPassword((shown) => !shown)}
					>
						{showPassword ? "Hide" : "Show"}
					</Button>
				</div>}
			</form.Field>

			{attempt.isError && <Alert color="red" title={mode === "register" ? "Could not create the account" : "Could not sign in"}>
				{String(attempt.error)}
			</Alert>}

			<Button type="submit" fullWidth loading={attempt.isPending}>
				{mode === "register" ? "Create account" : "Sign in"}
			</Button>

			<div className={classes.switch}>
				{mode === "register" ? "Already have an account?" : "No account yet?"}{" "}
				<Anchor component="button" type="button" size="sm" fw={600} onClick={() => setMode(mode === "register" ? "signIn" : "register")}>
					{mode === "register" ? "Sign in" : "Register"}
				</Anchor>
			</div>
		</Stack>
	</form>;
}
