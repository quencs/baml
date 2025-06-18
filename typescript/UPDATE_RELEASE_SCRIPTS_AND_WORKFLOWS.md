# Cursor Background Agent Prompt: Update Release Scripts & Workflows for TypeScript Monorepo Refactor

---

## Context

The TypeScript codebase has undergone a major refactor, moving to a modern monorepo structure. All packages are now under `typescript/packages/`, and all apps are under `typescript/apps/`. Shared UI components are in `typescript/packages/ui/`. Old, scattered directories and configs have been removed. All build, lint, and workspace configuration files have been updated to match the new structure.

## Your Task

Update all release process scripts and GitHub workflows to reflect this new structure. Ensure that all paths, build steps, and references to TypeScript packages or apps are correct and up-to-date.

## Key Details

- All reusable packages: `typescript/packages/*`
- All apps: `typescript/apps/*`
- Shared UI: `typescript/packages/ui/`
- Workspace tools/configs: `typescript/workspace-tools/`
- Old locations (e.g., `typescript/fiddle-frontend/`, `typescript/common/`, etc.) are now deleted or moved.
- See the migration reference table below for a summary of old vs. new locations.

## What to Update

1. **Release Scripts:**

   - Any scripts that build, test, lint, or publish TypeScript packages or apps.
   - Update all hardcoded paths to use the new structure.
   - Remove references to deleted/legacy locations.

2. **GitHub Workflows:**

   - All CI/CD workflows that build, test, lint, or release TypeScript code.
   - Update all steps, paths, and glob patterns to match the new monorepo layout.
   - Ensure that matrix builds, affected package detection, and deployment steps work with the new structure.

3. **Documentation in Scripts/Workflows:**
   - Update comments and documentation to reference the new structure and package/app names.

## Migration Reference Table

| Old Location                                | New Location                                 |
| ------------------------------------------- | -------------------------------------------- |
| `typescript/fiddle-frontend/`               | `typescript/apps/fiddle-web-app/`            |
| `typescript/vscode-ext/packages/vscode/`    | `typescript/apps/vscode-ext/`                |
| `typescript/vscode-ext/packages/web-panel/` | `typescript/apps/vscode-ext/src/web-panel/`  |
| `typescript/playground-common/`             | `typescript/packages/playground-common/`     |
| `typescript/common/`                        | `typescript/packages/common/`                |
| `typescript/fiddle-proxy/`                  | `typescript/packages/fiddle-proxy/`          |
| `typescript/nextjs-plugin/`                 | `typescript/packages/nextjs-plugin/`         |
| `typescript/codemirror-lang-baml/`          | `typescript/packages/codemirror-lang-baml/`  |
| `typescript/baml-schema-wasm-node/`         | `typescript/packages/baml-schema-wasm-node/` |
| `typescript/baml-schema-wasm-web/`          | `typescript/packages/baml-schema-wasm-web/`  |
| `typescript/language-server/`               | `typescript/packages/language-server/`       |

---

**Note:** This is a breaking change. All scripts and workflows must be updated to avoid referencing deleted or moved files. If you find any missing or misplaced files, please open a follow-up PR or issue.
