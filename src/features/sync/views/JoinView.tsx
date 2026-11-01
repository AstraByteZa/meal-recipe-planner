import { useState } from "react";
import { useForm } from "@tanstack/react-form";
import { useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { Alert, Button, Group, Stack, Text, Title } from "@mantine/core";
import { AuthLayout } from "../../../components/AuthLayout/AuthLayout";
import { InviteCodeInput } from "../components/InviteCodeInput";
import { fieldError } from "../components/formErrors";
import { useFamilyJoin, useFamilyPreview } from "../mutations";
import { syncStatusQueryOptions } from "../queries";
import { familyJoinSchema, type InvitePreview } from "../schema";
import { formatInviteCode } from "../format";
import { currentWeek } from "../../planner/week";
import classes from "./JoinView.module.css";

const pitch = {
	title: "Cook together.",
	text: "Join a family to share recipes and plan the week with the people you eat with.",
	points: ["Codes only, each works once", "Leave any time and keep your copy", "Only the owner can create invites"],
};

/** Two steps: check the code (which does not use it), then accept it. */
export function JoinView() {
	const navigate = useNavigate();
	const { data: status } = useQuery(syncStatusQueryOptions);
	const preview = useFamilyPreview();
	const join = useFamilyJoin();
	const [checked, setChecked] = useState<{ code: string; details: InvitePreview } | null>(null);

	const form = useForm({
		defaultValues: { inviteCode: "" },
		validators: { onChange: familyJoinSchema },
		onSubmit: ({ value }) => {
			const code = value.inviteCode;
			preview.mutate({ inviteCode: code }, {
				onSuccess: (details) => setChecked({ code, details }),
			});
		},
	});

	const back = () => navigate({ to: "/", search: { week: currentWeek() } });

	const accept = () => {
		if (!checked) return;
		join.mutate({ inviteCode: checked.code }, {
			onSuccess: () => navigate({ to: "/", search: { week: currentWeek() }, replace: true }),
		});
	};

	const decline = () => {
		setChecked(null);
		join.reset();
	};

	return <AuthLayout pitch={pitch}>
		{checked
			? <>
				<Text className={classes.eyebrow}>You're invited</Text>
				<div className={classes.family}>
					<span className={classes.initial}>{checked.details.family_name.slice(0, 1).toUpperCase()}</span>
					<Stack gap={2}>
						<Title order={2}>{checked.details.family_name}</Title>
						<Text size="sm" c="dimmed">Invited by {checked.details.invited_by}</Text>
					</Stack>
				</div>

				<div className={classes.members}>
					<div className={classes.avatars}>
						{checked.details.member_names.slice(0, 3).map((name, index) => <span key={`${name}-${index}`} className={classes.avatar}>
							{name.slice(0, 1).toUpperCase()}
						</span>)}
					</div>
					<Text size="sm">{checked.details.member_count} {checked.details.member_count === 1 ? "member" : "members"} already in</Text>
				</div>

				<Stack gap="xs">
					<Text size="sm">You will see the family's recipes and weekly plan.</Text>
					<Text size="sm">You can leave any time. Your copy of the recipes stays on this device as private data.</Text>
				</Stack>

				{join.isError && <Alert color="red" title="Could not join the family">{String(join.error)}</Alert>}

				<Button fullWidth onClick={accept} loading={join.isPending}>Accept and join</Button>
				<Button fullWidth variant="default" onClick={decline} disabled={join.isPending}>Decline</Button>
				<Text size="xs" c="dimmed" ta="center" ff="monospace">Code {formatInviteCode(checked.code)} · works once</Text>
			</>
			: <>
				<Stack gap={4}>
					<Title order={2}>Join a family</Title>
					<Text size="sm" c="dimmed">Ask the family owner for an invite code. Codes work once and expire after 7 days.</Text>
				</Stack>

				{status?.user && <Text size="sm" c="dimmed">Signed in as {status.user.email}</Text>}

				<form onSubmit={(event) => {
					event.preventDefault();
					form.handleSubmit();
				}}>
					<Stack gap="sm">
						<form.Field name="inviteCode">
							{(field) => <InviteCodeInput
								label="Invite code"
								description="Eight letters and numbers, shown as XXXX-XXXX."
								value={field.state.value}
								onChange={(value) => field.handleChange(value)}
								onBlur={field.handleBlur}
								error={fieldError(field) ?? (preview.isError ? String(preview.error) : undefined)}
							/>}
						</form.Field>
						<Button type="submit" fullWidth loading={preview.isPending}>Check code</Button>
					</Stack>
				</form>

				<Group justify="center">
					<Button variant="subtle" onClick={back}>Back</Button>
				</Group>
			</>}
	</AuthLayout>;
}
