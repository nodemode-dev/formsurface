export type DemoId = string;

export interface DemoInfo { id: DemoId; title: string; line: string; picture: string; command: string; label: string }

export const DEMOS: DemoInfo[] = [];
