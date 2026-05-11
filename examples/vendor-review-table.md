- Vendor Review Table #table @surface:columns [id:vendor-review]
  - How To Read This Map #guide [id:vendor-review/how-to]
    - Open the map in the TUI, press C, then press c inside Table View to choose fields
    - Useful columns include status owner priority fit cost risk next due
    - Treat each vendor row as a record; press Enter to jump back to the outline for notes
  - Shortlist #table @status:active [id:vendor-review/shortlist]
    - Northstar Fulfillment @status:active @owner:maya @priority:high @fit:strong @cost:medium @risk:low @next:demo @due:2026-05-18 [id:vendor-review/northstar]
      | Strong operations fit. Ask about exception handling before procurement review.
    - Harbor Packworks @status:active @owner:theo @priority:high @fit:medium @cost:low @risk:medium @next:references @due:2026-05-20 [id:vendor-review/harbor]
      | Price is attractive, but references need to confirm peak-season reliability.
    - Lumen Ship Studio @status:active @owner:jason @priority:medium @fit:strong @cost:high @risk:medium @next:pricing @due:2026-05-22 [id:vendor-review/lumen]
      | Best tooling and reporting. Cost may require a narrower first contract.
    - Orchard Logistics @status:watch @owner:maya @priority:medium @fit:medium @cost:medium @risk:high @next:security @due:2026-05-24 [id:vendor-review/orchard]
      | Needs a security review before it can move back into active consideration.
    - Canal House Supply @status:watch @owner:leah @priority:low @fit:weak @cost:low @risk:medium @next:hold @due:2026-06-01 [id:vendor-review/canal-house]
      | Keep as a fallback only if shortlist vendors fail procurement.
  - Decision Criteria #reference [id:vendor-review/criteria]
    - fit = how well the vendor matches the operating model
    - cost = relative contract and switching cost
    - risk = operational or compliance uncertainty
    - next = the immediate follow-up that should happen outside this table
