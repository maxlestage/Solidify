function wait(ms: number): Promise<number> { return new Promise((r) => setTimeout(() => r(ms), ms)); }
wait(10).then((v) => { console.log("then", v); return v * 2; }).then((v) => console.log("then2", v));
Promise.race([wait(30), wait(5)]).then((v) => console.log("race", v));
Promise.allSettled([wait(1), Promise.reject(new Error("no"))]).then((r) => console.log(r.length));
Promise.reject(new Error("bad")).catch((e) => console.log("caught", e.message));
