export interface ChatInputMention {
    type: "file";
    label: string;
    path: string;
}

export interface ChatInputValue {
    text: string;
    mentions: ChatInputMention[];
}

export interface ChatInputViewState {
    content: string;
    selectionStart: number;
    selectionEnd: number;
}
