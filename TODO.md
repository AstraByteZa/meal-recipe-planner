# TODO

Features to build later.

- [ ] Incorporate an image into a Recipe link
- [ ] Scrape a recipe website for the recipe and its ingredients

## Repository and releases

The repo is initialized locally (`git init -b main`) with no commits yet. The release workflow is `.github/workflows/release.yml`.

- [ ] Create an empty GitHub repo (for example `planner-app`). Do not add a README, .gitignore or licence there, since the local repo already has them.
- [ ] Make the first commit: `git add -A && git commit -m "Initial import"`.
- [ ] Add the remote and push: `git remote add origin git@github.com:<user>/planner-app.git && git push -u origin main`.
- [ ] Check the first Actions run for the Linux and Windows jobs. Fix any setup errors it reports.
- [ ] Publish a release: `git tag v0.1.0 && git push origin v0.1.0`. The workflow creates a draft release with the installers. Review it, then publish.
- [ ] Decide on Windows code signing. Unsigned installers show a SmartScreen warning.

## Open decisions and deviations

- [ ] Apply the week start setting (Monday or Sunday) to the planner's week helpers. It is saved in onboarding but not used yet.
- [ ] Use household size and dietary needs to filter and rank the "Auto-fill gaps" suggestions. They are saved but not used yet.
- [ ] Decide on the Snack track from the onboarding design. The app has no Snack track, and the seeded Breakfast Kids track is listed in onboarding.
- [ ] Give the starter ideas in onboarding real links, or keep them name-only. The design shows a source domain for each idea.
- [ ] Decide whether the first-plan step should pre-place the first recipe on Monday dinner, as the design shows. It currently starts empty.
- [ ] Decide whether existing databases should skip onboarding. Right now they open it on the next launch, because the onboarded flag defaults to off.
- [ ] Recipe delete wording: the design says past plans keep the meal name, but deleting a recipe removes its planned meals. Matching the design needs a schema change.
- [ ] Warn or merge when a pasted link is already saved. Pasting it again creates a second recipe.

## Cook now (see docs/plans/cook-now.md)

- [ ] Cook mode oven chips (temperature, fan setting, shelf). The data is only in the step text, so they are not in v1.
- [ ] Cook mode on mobile. The mobile mockup (`docs/designs/Cook now · mobile@1x.png`) is not built in v1.
- [ ] Settings pages for the `config` table (text size, keep awake, timer sound, and any later preferences).
- [ ] OS notifications when a cook timer finishes. v1 uses an in-app notification and a sound.
- [ ] Ingredient groups in cook mode and on the detail view (for example "Chicken and marinade"). Needs a data change.
- [ ] Verify the keep-awake approach on Linux (WebKitGTK) before choosing between the Wake Lock API and a Rust-side plugin.
