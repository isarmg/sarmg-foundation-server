/** Load and decode all registered application fonts once per document. */
export declare function prepareApplicationFonts(): Promise<void>;
/** Keep the page hidden on failure; start the UI after every font is loaded. */
export declare function startAfterFonts(start: () => void): Promise<void>;
