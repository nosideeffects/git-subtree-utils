import {readJson} from "std/fs/mod.ts";

const utf8TextDecoder = new TextDecoder("utf8");

(async function () {
    console.log(await getGitRoot());
})();

async function getGitRoot() {
    const process = await Deno.run({
        args: ["git", "rev-parse", "--show-toplevel"],
        stderr: "piped",
        stdin: "piped",
        stdout: "piped"
    });

    const output = await process.output();

    if ((await process.status()).code !== 0) {
        throw new Error("Not in a valid git repo");
    }

    return utf8TextDecoder.decode(output);
}
