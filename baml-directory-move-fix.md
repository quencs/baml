# BAML Language Server: Directory Move Issue Fix

## Problem Description

When moving the `baml_src` directory in a BAML project, the language server would encounter errors like:

```
WARN baml:main language_server::server::api::requests::hover: *** HOVER: Failed to find doc
ERROR baml:main language_server::server::api: An error occurred with result ID 87: File file:///old/path/baml_src/test-files/providers/openai.baml was not present in the project
ERROR baml:main language_server::server::api: An error occurred while running sync notification textDocument/didChange: Document controller not available
```

## Root Cause Analysis

The issue was in how the `BamlProjectManager` handles project lifecycle and file path resolution:

1. **Project Key Management**: Projects are stored in a `Map<string, Project>` where the key is the root path (the `baml_src` directory path).

2. **Stale Project References**: When a directory is moved:
   - The old project remains registered with the old path
   - Files at the new location try to find projects using the new path
   - `getProjectById()` fails because no project exists at the new path
   - File operations (hover, didChange, etc.) fail

3. **Inadequate Cleanup**: The file watcher would trigger `reload_project_files()` but wouldn't clean up the old project entry, leading to duplicate or orphaned projects.

## Solution Implementation

### 1. Enhanced Project Cleanup (`baml_project_manager.ts`)

Added `cleanupOrphanedProjects()` method that:
- Detects when files from an existing project now exist at a new location
- Identifies orphaned projects (projects at old paths)
- Removes stale project entries before creating new ones

```typescript
private cleanupOrphanedProjects(newRootPath: string, newFiles: [string, string][]) {
  // Implementation detects overlapping files between old and new locations
  // and removes orphaned project entries
}
```

### 2. Improved Project Resolution (`baml_project_manager.ts`)

Enhanced `getProjectById()` to:
- First try to find the project at the expected path
- If not found, search existing projects for files that might match
- Provide better logging for debugging move scenarios
- Handle edge cases gracefully

### 3. Resilient Request Handlers (`server.ts`)

Updated hover and content change handlers to:
- Gracefully handle missing projects
- Automatically attempt project reload when projects are not found
- Provide retry mechanisms for failed operations

**Hover Handler:**
```typescript
if (proj) {
  return proj.handleHoverRequest(doc, params.position)
} else {
  // Project not found - might be because directory was moved
  console.log(`Project not found for hover request: ${doc.uri}. Attempting to reload...`)
  bamlProjectManager.touch_project(URI.parse(doc.uri)).catch(...)
}
```

**Content Change Handler:**
```typescript
try {
  await bamlProjectManager.upsert_file(URI.parse(textDocument.uri), textDocument.getText())
} catch (e) {
  // Retry with project reload
  await bamlProjectManager.touch_project(URI.parse(textDocument.uri))
  await bamlProjectManager.upsert_file(URI.parse(textDocument.uri), textDocument.getText())
}
```

### 4. Enhanced File Watching (`server.ts`)

Improved the file watcher to:
- Process each changed file individually instead of just the first one
- Better filtering of relevant file changes
- Enhanced logging for debugging
- More robust error handling

## Benefits

1. **Seamless Directory Moves**: Moving `baml_src` directories no longer breaks language server functionality
2. **Automatic Recovery**: The system automatically detects and recovers from moved directories  
3. **Better Error Handling**: Graceful degradation when projects can't be found
4. **Improved Debugging**: Enhanced logging helps identify and troubleshoot move-related issues
5. **Reduced Error Spam**: Fewer error messages when directories are moved

## Testing Recommendations

To verify the fix works correctly:

1. **Basic Move Test**: 
   - Open a BAML project with language server running
   - Move the `baml_src` directory to a new location
   - Verify hover, completion, and other features continue working

2. **Multiple Projects Test**:
   - Have multiple BAML projects open
   - Move one project's `baml_src` directory
   - Verify other projects continue working normally

3. **File Operations Test**:
   - After moving a directory, try:
     - Hovering over symbols
     - Making edits to files
     - Saving files
     - Opening new files in the moved directory

## Future Improvements

- Consider implementing directory watching at the workspace level to proactively detect moves
- Add configuration option to disable automatic project recovery if needed
- Implement more sophisticated project migration for complex scenarios
- Add telemetry to track directory move scenarios for further optimization