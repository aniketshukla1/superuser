/** Someone who recently messaged the app's messaging bot, and whether they may use it. */
export interface RecentSender {
  target: string;
  authorized: boolean;
  user_id: string;
  user_name: string | null;
  chat_id: string;
  chat_type: string;
  /** The workspace, when the message came through the managed relay; null over manual Socket Mode. */
  team_id?: string | null;
}
