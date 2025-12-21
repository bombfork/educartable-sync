# Educartable Downloader

## Project Overview

This project provides a batch download feature for pictures from the Educartable website (https://app.educartable.com), allowing parents to synchronize photos of their children from the school platform to a local folder.

## Problem Statement

- **Website**: https://app.educartable.com
- **Purpose**: School teachers publish articles and pictures about students' activities, allowing parents to stay informed
- **Current Limitation**: The website only allows downloading pictures one at a time
- **Missing Feature**: No batch download capability

## Project Goals

Create a tool that enables users to:
1. Select a local folder on their computer
2. Synchronize (download-only, one-way) all pictures they have access to from the Educartable website
3. Automate the batch download process

## Constraints

- User access level only (no admin/special privileges)
- Must work within the constraints of regular user authentication
- One-way synchronization (download only, no uploads)

## Technical Approach

**Technology Stack**: Tauri v2 (Rust backend + HTML/CSS/JS frontend)
**Authentication**: Webview-based OAuth (Keycloak)
**Target Platforms**: Linux, macOS, Windows

See [DISCOVERY.md](./DISCOVERY.md) for complete API and authentication findings.

## Project Management

### GitHub Issues Workflow

This project uses GitHub issues for all project management and task tracking.

### Issue Sizing System

All issues must be labeled with a size estimate:

- **`size-s`**: Less than half a day of work for 1 engineer
- **`size-m`**: Less than a day of work for 1 engineer
- **`size-l`**: More than a day of work for 1 engineer

### Issue Management Rules

1. **Unsized issues**: Any issue without a size label must be sized before work begins
2. **Large issues (`size-l`)**: Must be split into smaller `size-m` and/or `size-s` issues
   - The original `size-l` issue becomes the parent
   - Reference child issues using GitHub task lists
3. **Medium issues (`size-m`)**: Should be split into smaller `size-s` issues
   - The original `size-m` issue becomes the parent
   - Reference child issues using GitHub task lists
4. **Implementation**: **Only `size-s` issues should be implemented**
   - This ensures work is broken down into manageable chunks
   - Makes progress tracking more granular and accurate

### Issue Hierarchy Example

```
Issue #1 (size-l): Implementation Plan
  ├── Issue #2 (size-m): Authentication Module
  │   ├── Issue #3 (size-s): Create webview window
  │   ├── Issue #4 (size-s): Implement navigation monitoring
  │   └── Issue #5 (size-s): Add token extraction via JS injection
  └── Issue #6 (size-m): API Client Module
      ├── Issue #7 (size-s): Implement user info endpoint
      ├── Issue #8 (size-s): Implement activities pagination
      └── Issue #9 (size-s): Implement signed URL retrieval
```

### Working on Issues

1. Find or create an issue
2. Ensure it has a size label
3. If `size-m` or `size-l`, break it down into child issues
4. Only work on `size-s` issues
5. Reference parent issue in child issues using "Part of #N"
6. Update parent issue task list as child issues are completed
