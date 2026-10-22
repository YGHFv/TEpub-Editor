import { describe, expect, it } from "vitest";
import {
    PHONE_HISTORY_LIMIT,
    phoneBackTarget,
    recordPhoneNavigation,
    routeHref,
} from "./phoneHistory";

describe("mobile app history", () => {
    it("keeps query strings and hashes in the route identity", () => {
        expect(
            routeHref({
                pathname: "/reader",
                search: "?file=a.epub",
                hash: "#chapter-2",
            }),
        ).toBe("/reader?file=a.epub#chapter-2");
    });

    it("starts a stack on enter and appends ordinary navigation", () => {
        let history = recordPhoneNavigation([], "/toolbox", "enter");
        history = recordPhoneNavigation(history, "/editor", "link");
        history = recordPhoneNavigation(history, "/reader", "goto");

        expect(history).toEqual(["/toolbox", "/editor", "/reader"]);
    });

    it("preserves A -> B -> A so Back returns to B", () => {
        const history = ["/a", "/b", "/a"];

        expect(phoneBackTarget(history, "/a")).toBe("/b");
        expect(recordPhoneNavigation(history, "/b", "popstate")).toEqual([
            "/a",
            "/b",
        ]);
    });

    it("uses a popstate target that is already in the app stack", () => {
        const history = ["/toolbox", "/library", "/reader"];

        expect(
            recordPhoneNavigation(history, "/toolbox", "popstate"),
        ).toEqual(["/toolbox"]);
    });

    it("resets the stack on a primary tab", () => {
        expect(
            recordPhoneNavigation(
                ["/toolbox", "/reader"],
                "/library",
                "goto",
                true,
            ),
        ).toEqual(["/library"]);
    });

    it("does not grow past the limit", () => {
        let history: string[] = [];
        for (let index = 0; index < PHONE_HISTORY_LIMIT + 5; index += 1) {
            history = recordPhoneNavigation(
                history,
                `/tool-${index}`,
                "link",
            );
        }

        expect(history).toHaveLength(PHONE_HISTORY_LIMIT);
        expect(history[0]).toBe("/tool-5");
    });
});
