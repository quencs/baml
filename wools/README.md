# WOOL — Working On Our Language 🐑

WOOLs are design proposals for evolving the BAML language and core tooling.

Below is an auto-generated index of all WOOLs.

> ⚠️ Do not edit the table below by hand.
> Run the WOOL update script instead (see instructions at the bottom).

<!-- WOOL-TABLE-START -->

| Status | Meaning |
| :--- | :--- |
| <img src="https://img.shields.io/badge/Status-Draft-lightgrey" alt="Draft"> | Work in progress, not ready for review |
| <img src="https://img.shields.io/badge/Status-Proposed-yellow" alt="Proposed"> | Ready for review and discussion |
| <img src="https://img.shields.io/badge/Status-Accepted-brightgreen" alt="Accepted"> | Approved for implementation |
| <img src="https://img.shields.io/badge/Status-Implemented-blue" alt="Implemented"> | Feature is live in BAML |
| <img src="https://img.shields.io/badge/Status-Rejected-red" alt="Rejected"> | Decided against |
| <img src="https://img.shields.io/badge/Status-Superseded-orange" alt="Superseded"> | Replaced by another WOOL |

<table>
  <thead>
    <tr>
      <th>WOOL</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td><a href="./proposals/WOOL-001-exceptions/README.md"><strong>WOOL-001</strong>: Exceptions</a> &nbsp; <img src="https://img.shields.io/badge/Status-Draft-lightgrey" alt="Draft"><br><br>This is a placeholder for the Exceptions design proposal.<br><br><span style='font-size:0.8em; color:gray'>Shepherd(s): Vaibhav Gupta <vbv@boundaryml.com> | Created: 2025-11-20 | Updated: 2025-11-20</span></td>
    </tr>
  </tbody>
</table>

<!-- WOOL-TABLE-END -->

---

## Management

> Scripts are self-contained Python scripts using [`uv`](https://github.com/astral-sh/uv). Ensure `uv` is installed.

### Creating a new WOOL

To create a new proposal:
```bash
mise run wool:new -- "Feature Name"
```

This will:
1. Create a new directory `wools/WOOL-XXX-feature-name/`
2. Create a `README.md` template inside it with the next available WOOL ID.

### Updating the Index

After modifying any WOOL, update this README table:
```bash
mise run wool:readme
```

### Managing WOOLs

To update a WOOL's status or timestamp:

**Touch (Update Timestamp):**
```bash
mise run wool:update 001
# OR
mise run wool:update WOOL-001-exceptions
```

**Change Status:**
```bash
mise run wool:update 001 --status Proposed
```
(Valid statuses: Draft, Proposed, Accepted, Implemented, Rejected, Superseded)

