// Neither package ships types, and a strict tsc refuses to import them without one.
declare module 'path-browserify' {
    export function basename(path: string, ext?: string): string;
    export function dirname(path: string): string;
    export function extname(path: string): string;
    export function join(...parts: string[]): string;
    export function normalize(path: string): string;
    export function resolve(...parts: string[]): string;
    export function isAbsolute(path: string): boolean;
    export function relative(from: string, to: string): string;
    export const sep: string;
}

declare module 'await-notify' {
    export class Subject {
        wait(timeout?: number): Promise<void>;
        notify(): void;
        notifyAll(): void;
    }
}
