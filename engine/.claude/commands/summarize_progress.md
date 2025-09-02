---
name: summarize_progress
description: Analyze active session progress on implementation plans and create incremental sprint plans
---

# Progress Summary Command

You are tasked with analyzing progress on implementation plans by examining the active session history and creating incremental sprint plans. This command helps track what's been completed in the current session, what stage we're at, and what needs to be done next.

## Command Usage

This command takes an implementation plan file as input:

```
/summarize_progress path/to/implementation_plan.md
```

The command will analyze the active session history to understand progress and create an incremental sprint plan.

## Initial Response

When this command is invoked:

1. **Check if parameters were provided**:
   - If no implementation plan file path is provided, respond with:
   ```
   I'll help you summarize progress on an implementation plan. Please provide the path to the plan file:

   /summarize_progress path/to/implementation_plan.md

   For example: /summarize_progress thoughts/shared/plans/feature_abc.md
   ```

2. **If a file path is provided**:
   - Immediately read the implementation plan file FULLY
   - Analyze the active session history to understand what's been accomplished
   - Begin the progress analysis process

## Process Steps

### Step 1: Plan Analysis

1. **Read the implementation plan completely**:
   - Use the Read tool WITHOUT limit/offset parameters to read the entire plan
   - Identify all phases, tasks, and success criteria
   - Note the file paths and components mentioned in the plan

2. **Extract key information from the plan**:
   - Overview and scope
   - Implementation phases and their descriptions
   - Specific files and components to be modified
   - Success criteria (both automated and manual)
   - Any constraints or dependencies mentioned

### Step 2: Current State Investigation

1. **Spawn research tasks to assess current implementation state**:
   - Use the **progress-reviewer** agent to examine the current state of files mentioned in the plan
   - Check for test files, configuration changes, and documentation updates

2. **Analyze the current codebase against plan requirements**:
   - Compare what exists now vs. what was planned
   - Identify completed work, work in progress, and untouched areas
   - Check if any planned changes have been implemented differently than specified

### Step 3: Progress Assessment

1. **Evaluate each phase against current state**:
   - **Completed**: All success criteria met, code implemented as planned
   - **In Progress**: Some work done but not complete
   - **Not Started**: No work begun on this phase
   - **Modified**: Work done but differs from original plan

2. **Check automated success criteria**:
   - Run or verify the commands specified in the plan
   - Check if tests pass, linting succeeds, etc.
   - Verify file existence and content where applicable

3. **Assess manual verification items**:
   - Note which items require human testing
   - Identify any that may have been completed but not documented

### Step 4: Progress Summary Creation

Create a comprehensive progress summary that includes:

```markdown
# Progress Summary: [Feature/Task Name]

## Plan Overview
[Brief description from the original plan]

## Overall Progress
- **Total Phases**: [X]
- **Completed**: [X] phases
- **In Progress**: [X] phases  
- **Not Started**: [X] phases
- **Overall Completion**: [X]%

## Phase-by-Phase Status

### Phase 1: [Phase Name] - [Status]
**Status**: [Completed/In Progress/Not Started/Modified]

**Progress Details**:
- [Specific task] - [Status with details]
- [Another task] - [Status with details]

**Success Criteria Status**:
#### Automated Verification:
- [ ] [Criteria] - [Status: Pass/Fail/Not Tested]
- [ ] [Criteria] - [Status: Pass/Fail/Not Tested]

#### Manual Verification:
- [ ] [Criteria] - [Status: Complete/Incomplete/Not Tested]
- [ ] [Criteria] - [Status: Complete/Incomplete/Not Tested]

**Notes**: [Any deviations from plan, issues encountered, etc.]

---

### Phase 2: [Phase Name] - [Status]
[Similar structure...]

## Key Findings

### Completed Work
- [List of major accomplishments]
- [Files successfully modified/created]
- [Tests passing, etc.]

### Work In Progress
- [What's currently being worked on]
- [Partial implementations]
- [Blockers or challenges]

### Remaining Work
- [What still needs to be done]
- [Dependencies that need to be resolved]
- [Estimated effort remaining]

### Deviations from Plan
- [Any changes made that differ from the original plan]
- [New requirements discovered during implementation]
- [Technical debt or refactoring needed]

## Recommendations

### Immediate Next Steps
1. [Priority action item]
2. [Another priority item]
3. [Blockers to resolve]

### Risk Assessment
- **High Risk**: [Issues that could derail the project]
- **Medium Risk**: [Challenges that need attention]
- **Low Risk**: [Minor concerns to monitor]


## References
- Original Plan: [file path]
- Related Files: [list of key files involved]
- Tests: [test files and their status]
```

### Step 5: Creation and Naming

1. **Determine progress summary filename**:
   - Extract the base name from the input plan (e.g., `impl_abc` from `impl_abc.md`)
   - Check if `impl_abc_1.md` exists
   - If it exists, create `impl_abc_2.md`
   - If it doesn't exist, create `impl_abc_1.md`
   - Continue incrementing for subsequent sprints

2. **Create the progress summary file**:
   - Write theprogress summary to `./claude/progress/impl_abc_[sprint_number].md`
   - Include all the analysis and recommendations from previous steps
   - Focus on what needs to be accomplished in this specific sprint


## Important Guidelines

1. **Be Accurate**:
   - Verify actual code state, don't assume
   - Run commands when possible to check success criteria
   - Cross-reference multiple sources of information

2. **Be Actionable**:
   - Provide clear next steps
   - Identify blockers and dependencies
   - Give specific recommendations for moving forward

3. **Be Honest**:
   - Don't sugar-coat progress issues
   - Highlight risks and challenges
   - Acknowledge when plans need adjustment

4. **Track Changes**:
   - Note any deviations from the original plan
   - Document new requirements discovered
   - Suggest plan updates when appropriate

## Success Criteria for Progress Summary

A good progress summary should:

- **Accurately reflect current state** based on actual code analysis
- **Provide clear status** for each phase and major task
- **Identify actionable next steps** with clear ownership
- **Highlight risks and blockers** that need attention
- **Suggest plan adjustments** when the original plan is outdated
- **Include specific file references** and code examples where relevant
- **Separate automated vs manual verification** clearly
- **Provide overall completion percentage** and timeline impact assessment

## Example Interaction Flow

```
User: /summarize_progress thoughts/shared/plans/user_authentication.md
Assistant: I'll analyze the progress on the user authentication implementation plan. Let me read the plan file and assess the current state...

[Reads plan file completely]

Based on my analysis of the plan and current codebase state, here's the progress summary:

# Progress Summary: User Authentication System
[Complete summary follows...]

I've created ./.claude/progress/user_authentication_1.md 
```
