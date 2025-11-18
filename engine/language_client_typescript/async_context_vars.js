import { BamlSpan } from './native.js';
import { AsyncLocalStorage } from 'async_hooks';
export class BamlCtxManager {
    rt;
    ctx;
    constructor(rt) {
        this.rt = rt;
        this.ctx = new AsyncLocalStorage();
        this.ctx.enterWith(rt.createContextManager());
        process.on('exit', () => {
            this.rt.flush();
        });
    }
    allowResets() {
        let store = this.ctx.getStore();
        if (store === undefined) {
            return true;
        }
        if (store.contextDepth() > 0) {
            return false;
        }
        return true;
    }
    reset() {
        this.ctx = new AsyncLocalStorage();
        this.ctx.enterWith(this.rt.createContextManager());
    }
    upsertTags(tags) {
        const manager = this.ctx.getStore();
        manager.upsertTags(tags);
    }
    cloneContext() {
        let store = this.ctx.getStore();
        if (store === undefined) {
            store = this.rt.createContextManager();
            this.ctx.enterWith(store);
        }
        return store.deepClone();
    }
    startTrace(name, args, envVars) {
        const mng = this.cloneContext();
        return [mng, BamlSpan.new(this.rt, name, args, mng, envVars)];
    }
    endTrace(span, response, envVars) {
        const manager = this.ctx.getStore();
        if (!manager) {
            console.error('Context lost before span could be finished\n');
            return;
        }
        try {
            span.finish(response === undefined ? null : response, manager, envVars);
        }
        catch (e) {
            console.error('BAML internal error', e);
        }
    }
    flush() {
        this.rt.flush();
    }
    onLogEvent(callback) {
        if (!callback) {
            this.rt.setLogEventCallback(undefined);
            return;
        }
        this.rt.setLogEventCallback((error, param) => {
            if (!error) {
                callback(param);
            }
        });
    }
    traceFnSync(name, func) {
        return ((...args) => {
            const params = args.reduce((acc, arg, i) => ({
                ...acc,
                [`arg${i}`]: arg, // generic way to label args
            }), {});
            const [mng, span] = this.startTrace(name, params, process.env);
            return this.ctx.run(mng, () => {
                try {
                    const response = func(...args);
                    this.endTrace(span, response, process.env);
                    return response;
                }
                catch (e) {
                    this.endTrace(span, e, process.env);
                    throw e;
                }
            });
        });
    }
    traceFnAsync(name, func) {
        const funcName = name;
        return (async (...args) => {
            const params = args.reduce((acc, arg, i) => ({
                ...acc,
                [`arg${i}`]: arg, // generic way to label args
            }), {});
            const [mng, span] = this.startTrace(name, params, process.env);
            return await this.ctx.run(mng, async () => {
                try {
                    const response = await func(...args);
                    this.endTrace(span, response, process.env);
                    return response;
                }
                catch (e) {
                    this.endTrace(span, e, process.env);
                    throw e;
                }
            });
        });
    }
}
