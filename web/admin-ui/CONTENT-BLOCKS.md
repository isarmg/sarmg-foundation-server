# Default content-block appearance and consumer customization

Source: union-rust's Apache-2.0 `xcss-design` content cards, login card, and palette.
The default `@xcss/web/admin-ui/styles.css` imports this stylesheet automatically.
It applies when the document root has no `data-xcss-appearance` attribute or its value is `content-blocks`.
Consumers can opt out with `<html data-xcss-appearance="custom">` or another custom name,
then load their own CSS after the base styles; all content-block rules and the palette leave scope.
This does not replace authentication components or change APIs, sessions, permissions, or default fonts.
The separate `@xcss/web/admin-ui/content-blocks.css` export also supports native ESM pages.

Login reuses the current AdminShell semantic form: a 380px, 3:2 card with six rows, transparent inputs, and text actions.
Assistive technology still receives the title and product identity. The error row scrolls without removing the full request ID.
It follows the existing `data-theme="light|dark"` and system theme, preserving forced colors and keyboard focus.

Products can compose six-row 3:2 cards with `.xcss-content-grid`, `.xcss-content-card`,
`.xcss-content-card__inner`, and `.xcss-content-row`.
Long forms, details, and tables use the expandable `.xcss-content-panel`, rather than a fixed-ratio card.
Standard tables align cell contents and action groups to the left, with the first and last columns aligned to the content edges.
Tables nested inside `.xcss-content-panel` reuse its background and padding without adding another card layer.
Hiding a table also hides its scroll region, so an empty region does not retain layout spacing.
Text buttons align with content edges while retaining a minimum 44px hit target.
These classes define presentation only and contain no client or server behavior.

Spacing from the menu bar to the first content row, between standard content-block rows, and from preceding content to a subheading
uses `--xcss-content-spacing`, whose default is `--xcss-space-4` (16px). Ordinary vertical product containers can use
`.xcss-content-stack`; it creates grid gaps with this variable and clears block margins on direct children.
Products needing a different density can override the variable in their own scope instead of adding separate ad hoc heading and content margins.

## Current release integration

See the root README for the current package version. Each product's manifest, lockfile, and full source revision determine the version it consumes.
Import `@xcss/web/admin-ui/styles.css` directly; the package includes the default content-block CSS. Consumers pin the release archive URL and lockfile integrity digest. Independent builds require no neighboring xcss source.

xcss owns these presentation classes, default design tokens, and accessibility behavior. Products own card fields, statistical rules, instance actions, chart data, and business events. Product DTOs, endpoints, error codes, and product-name branches do not belong in this package.

Consumers can customize the default appearance; it does not impose a brand on external products. `@xcss/web/web-fonts` supplies the fonts.
Later changes require a new immutable package and updated consumer lockfiles, without overwriting existing release archives.
See the [1.0.0 release notes](../../docs/releases/1.0.0.md) for current table content boundaries and account appearance.
