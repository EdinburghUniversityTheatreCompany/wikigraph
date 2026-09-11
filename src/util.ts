export const hookLoad = <T>(f: () => T): Promise<T> => {
    if (document.readyState === "loading") {
        return new Promise((res) => document.addEventListener("DOMContentLoaded", () => res(f())));
    } else {
        return Promise.resolve(f());
    }
};
