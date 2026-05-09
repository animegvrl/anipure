Array.from(document.querySelectorAll(".entry-type-file")).map((e) => e.querySelector("a").href);

animeFolder.filter((f) => !f.includes("info.txt"))
                            .map((file, fileIdx) => `#EXTINF:${fileIdx + 1}\n${file}`)
           .join('\n');
