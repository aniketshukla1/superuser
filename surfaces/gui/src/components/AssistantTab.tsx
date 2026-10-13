// The Assistant tab — the persona's home in the panel.
//
// The organising idea, taken from the reference screenshots: adding a workflow or a
// schedule does NOT open a form. It asks a question and offers a few concrete
// starting points, and answering it happens in the chat with the assistant that will
// run the thing. Configuration becomes conversation, which is why the panel reads as
// the product rather than as a settings screen.
//
// So every "+ Add" here composes a prompt and sends it. Nothing on this tab writes a
// config record behind the user's back.

export { AssistantTab } from "./AssistantTab/AssistantTab";
