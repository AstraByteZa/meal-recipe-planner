import { useState } from "react";
import { useForm } from "@tanstack/react-form";
import { Accordion, Alert, Anchor, Button, Stack, TextInput } from "@mantine/core";
import { z } from "zod";
import { useSyncLogin, useSyncRegister } from "../mutations";
import { accountFormSchema } from "../schema";
import { fieldError } from "./formErrors";
import classes from "./AccountForm.module.css";

interface AccountFormProps {
	/** A custom address to pre-fill. Blank when the device uses the built-in server. */
	customServerUrl: string;
	/** The built-in server of this build, if it has one. */
	defaultServerUrl: string | null;
	/** Whether Advanced Options starts open. */
	startOpen: boolean;
	onSignedIn?: () => void;
}

const formSchema = accountFormSchema.extend({ displayName: z.string() });

/**
 * Sign in or register. Used on the sign-in page and in the sync settings modal, so both behave the
 * same way. Register adds an optional display name. The server address is optional and lives in
 * Advanced Options.
 */
export function AccountForm({ customServerUrl, defaultServerUrl, startOpen, onSignedIn }: AccountFormProps) {
	const [mode, setMode] = useState<"signIn" | "register">("signIn");
	const [showPassword, setShowPassword] = useState(false);
	const login = useSyncLogin();
	const register = useSyncRegister();
	const attempt = mode === "register" ? register : login;

	const form = useForm({
		defaultValues: { serverUrl: customServerUrl, displayName: "", email: "", password: "" },
		validators: { onChange: formSchema },
		onSubmit: ({ value }) => {
			const account = {
				// Blank sends no address, so the app uses the built-in server.
				serverUrl: value.serverUrl.trim() || undefined,
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
			<Accordion variant="contained" defaultValue={startOpen ? "advanced" : null}>
				<Accordion.Item value="advanced">
					<Accordion.Control>Advanced Options</Accordion.Control>
					<Accordion.Panel>
						<form.Field name="serverUrl">
							{(field) => <TextInput
								label="Server address"
								description={defaultServerUrl
									? `Leave blank to use ${defaultServerUrl}. Enter your own server's address to self-host.`
									: "This build has no built-in server. Enter your server's address."}
								placeholder="https://sync.example.com"
								value={field.state.value}
								onChange={(event) => field.handleChange(event.currentTarget.value)}
								onBlur={field.handleBlur}
								error={fieldError(field)}
							/>}
						</form.Field>
					</Accordion.Panel>
				</Accordion.Item>
			</Accordion>

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
