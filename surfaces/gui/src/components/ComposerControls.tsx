// The composer's control bar: mode, attach, microphone, model, effort, context.
//
// They live together in one file because they are one row and they share a shape. Every
// one of them is a quiet chip that opens a panel of rows, and the rows agree about their
// parts — a number you can press, a name, a line saying what it does, a mark on the one
// that is current. Six near-identical menus written six times is how they stop agreeing.

export { ChipButton, Menu, MenuHeading, MenuDivider, MenuRow } from "./ComposerControls/MenuRow";
export { ModeMenu } from "./ComposerControls/ModeMenu";
export type { ModeOption } from "./ComposerControls/ModeMenu";
export { AttachMenu } from "./ComposerControls/AttachMenu";
export { MicControl } from "./ComposerControls/MicControl";
export { ModelMenu } from "./ComposerControls/ModelMenu";
export { EffortChip } from "./ComposerControls/EffortChip";
export { ContextRing } from "./ComposerControls/ContextRing";
