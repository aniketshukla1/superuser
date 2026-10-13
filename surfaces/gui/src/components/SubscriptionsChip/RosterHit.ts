// A workspace roster hit for the typeahead: type a channel NAME, we resolve the
// id (conversations.list, cached on the desktop) and compose the address.
export interface RosterHit {
  address: string;
  name: string;
  workspace: string; // labeled only when >1 workspace is connected
  is_private: boolean;
  is_member: boolean;
}
