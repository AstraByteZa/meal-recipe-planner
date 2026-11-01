import { TextInput, type TextInputProps } from "@mantine/core";
import { formatInviteCode, normalizeInviteCode } from "../format";
import classes from "./InviteCodeInput.module.css";

interface InviteCodeInputProps extends Omit<TextInputProps, "value" | "onChange"> {
	/** The stored value: upper case letters and digits, no dash. */
	value: string;
	onChange: (value: string) => void;
}

/** A monospace code field. It shows `XXXX-XXXX` and stores the characters without the dash. */
export function InviteCodeInput({ value, onChange, ...props }: InviteCodeInputProps) {
	return <TextInput
		{...props}
		value={formatInviteCode(value)}
		onChange={(event) => onChange(normalizeInviteCode(event.currentTarget.value))}
		autoComplete="off"
		autoCapitalize="characters"
		spellCheck={false}
		classNames={{ input: classes.input }}
	/>;
}
