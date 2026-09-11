# WabiSabi UI Consumer Playbook

## Status

This is a project rule for all Kineto Flutter UI work.

Kineto is a consumer of WabiSabi:

- public dependency: `https://github.com/bayanasar/wabisabi.git`
- WabiSabi owns the reusable Flutter UI toolkit, themes, design tokens, and shared visual primitives
- Kineto owns product content, product state, screen composition, workflow data, and project-specific presentation values

The goal is one evolving UI system, not a WabiSabi-inspired fork inside Kineto.

## Hard rule

All reusable Flutter UI primitives in Kineto must come from WabiSabi.

Kineto must not maintain a parallel local design system. In particular, do not add local copies or replacements for:

- theme definitions
- palette or semantic color tokens
- spacing tokens
- typography systems
- button/card/panel/scaffold primitives
- reusable navigation, status, form, or feedback widgets
- toolkit-level visual behavior already owned by WabiSabi

Do not copy WabiSabi source into Kineto and modify it locally.

## What may remain in Kineto

Kineto screen code may contain:

- product copy and labels
- project/workflow data
- state and event handling
- screen composition and feature-specific layout
- one-off dimensions needed by a specific screen or media preview
- numeric/color values that are themselves project data or content
- Flutter layout primitives such as `Row`, `Column`, `Stack`, `Padding`, `SizedBox`, and constraints when they describe composition rather than a reusable design primitive

A useful test is: if the same visual rule or widget would plausibly be useful to another WabiSabi consumer, it belongs in WabiSabi, not Kineto.

Reusable visual constants must become WabiSabi tokens instead of accumulating in Kineto.

## Theme and tokens

Kineto must obtain application theme and reusable visual tokens from WabiSabi.

Use `WabTheme` for Flutter theme construction. Do not build a project-local `ThemeData` system or create Kineto-specific token files that shadow WabiSabi.

Feature code should prefer WabiSabi components and theme values over raw Material/Cupertino styling.

## Missing-widget rule

If Kineto needs a reusable UI widget that WabiSabi does not provide:

1. Do not implement a private Kineto version as the permanent solution.
2. Add the primitive/component to WabiSabi through its canonical internal contribution workflow.
3. Add or update WabiSabi tests as appropriate.
4. Update the WabiSabi `example/` application so the new widget is visible and exercised as part of the toolkit showcase.
5. Let the public WabiSabi mirror receive the change.
6. Update Kineto's WabiSabi dependency and consume the upstream component.

A short-lived Kineto prototype is acceptable only when needed to discover the correct API. It must not become a second maintained widget implementation; the final reusable implementation moves upstream.

## Organic toolkit development

Kineto is expected to help WabiSabi develop organically.

Consumer pressure is useful design input. Missing primitives, awkward APIs, missing tokens, desktop-specific gaps, accessibility gaps, and state patterns discovered while building Kineto should be treated as WabiSabi product feedback.

The desired loop is:

```text
Kineto need
   ↓
identify reusable UI primitive
   ↓
implement/fix in WabiSabi
   ↓
update WabiSabi example + tests
   ↓
public mirror/update
   ↓
upgrade Kineto consumer
```

Do not work around a toolkit gap indefinitely in Kineto when fixing WabiSabi would benefit every consumer.

## Dependency rule

The committed Kineto dependency uses the public repository:

```yaml
wabisabi:
  git:
    url: https://github.com/bayanasar/wabisabi.git
    ref: main
```

`pubspec.lock` pins the resolved commit for reproducible builds. Updating the toolkit is an explicit consumer action (`flutter pub upgrade wabisabi`) after an upstream change is ready.

The public repository is the dependency surface, not the contribution target. WabiSabi source changes are authored/reviewed through its canonical internal forge workflow and mirrored outward.

## Local upstream development without leaking internal topology

When Kineto and an unreleased WabiSabi change must be tested together, use an untracked `pubspec_overrides.yaml` with a local checkout path, for example:

```yaml
dependency_overrides:
  wabisabi:
    path: <local-wabisabi-checkout>
```

Never commit the override or an internal forge URL/path to Kineto. The committed dependency remains the public GitHub URL.

## Review checklist

A Flutter UI change is not ready if any of the following is true:

- it introduces a local theme/token system
- it duplicates a WabiSabi widget
- it adds reusable styling directly to Kineto instead of upstreaming it
- a newly added WabiSabi component lacks an example/showcase update
- Kineto depends on an internal repository URL or local filesystem path in committed files
- a toolkit upgrade changes UI without the consumer being intentionally updated and tested

The default review question is: **is this Kineto product code, or is this actually WabiSabi toolkit code?**
