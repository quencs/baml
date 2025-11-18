import { TypeBuilder as _TypeBuilder, } from './native.js';
export class TypeBuilder {
    tb;
    classes;
    enums;
    runtime;
    constructor({ classes, enums, runtime }) {
        this.classes = classes;
        this.enums = enums;
        this.tb = new _TypeBuilder();
        this.runtime = runtime;
    }
    reset() {
        this.tb.reset();
    }
    _tb() {
        return this.tb;
    }
    null() {
        return this.tb.null();
    }
    string() {
        return this.tb.string();
    }
    literalString(value) {
        return this.tb.literalString(value);
    }
    literalInt(value) {
        return this.tb.literalInt(value);
    }
    literalBool(value) {
        return this.tb.literalBool(value);
    }
    int() {
        return this.tb.int();
    }
    float() {
        return this.tb.float();
    }
    bool() {
        return this.tb.bool();
    }
    list(type) {
        return this.tb.list(type);
    }
    map(keyType, valueType) {
        return this.tb.map(keyType, valueType);
    }
    union(types) {
        return this.tb.union(types);
    }
    classViewer(name, properties) {
        return new ClassViewer(this.tb, name, new Set(properties));
    }
    classBuilder(name, properties) {
        return new ClassBuilder(this.tb, name, new Set(properties));
    }
    enumViewer(name, values) {
        return new EnumViewer(this.tb, name, new Set(values));
    }
    enumBuilder(name, values) {
        return new EnumBuilder(this.tb, name, new Set(values));
    }
    addClass(name) {
        if (this.classes.has(name)) {
            throw new Error(`Class ${name} already exists`);
        }
        if (this.enums.has(name)) {
            throw new Error(`Enum ${name} already exists`);
        }
        this.classes.add(name);
        return new ClassBuilder(this.tb, name);
    }
    addEnum(name) {
        if (this.classes.has(name)) {
            throw new Error(`Class ${name} already exists`);
        }
        if (this.enums.has(name)) {
            throw new Error(`Enum ${name} already exists`);
        }
        this.enums.add(name);
        return new EnumBuilder(this.tb, name);
    }
    addBaml(baml) {
        this.tb.addBaml(baml, this.runtime);
    }
}
export class ClassAst {
    properties;
    bldr;
    constructor(tb, name, properties = new Set()) {
        this.properties = properties;
        this.bldr = tb.getClass(name);
    }
    listProperties() {
        return this.bldr.listProperties();
    }
    type() {
        return this.bldr.field();
    }
}
export class ClassViewer extends ClassAst {
    constructor(tb, name, properties = new Set()) {
        super(tb, name, properties);
    }
    listProperties() {
        return Array.from(this.properties).map((name) => [name, new ClassPropertyViewer()]);
    }
    property(name) {
        if (!this.properties.has(name)) {
            throw new Error(`Property ${name} not found.`);
        }
        return new ClassPropertyViewer();
    }
}
export class ClassBuilder extends ClassAst {
    constructor(tb, name, properties = new Set()) {
        super(tb, name, properties);
    }
    addProperty(name, type) {
        if (this.properties.has(name)) {
            throw new Error(`Property ${name} already exists.`);
        }
        this.properties.add(name);
        return new ClassPropertyBuilder(this.bldr.property(name).setType(type));
    }
    listProperties() {
        return this.bldr.listProperties();
    }
    removeProperty(name) {
        this.properties.delete(name);
        this.bldr.removeProperty(name);
    }
    reset() {
        this.bldr.reset();
    }
    property(name) {
        if (!this.properties.has(name)) {
            throw new Error(`Property ${name} not found.`);
        }
        return new ClassPropertyBuilder(this.bldr.property(name));
    }
}
class ClassPropertyViewer {
    constructor() { }
}
class ClassPropertyBuilder {
    bldr;
    constructor(bldr) {
        this.bldr = bldr;
    }
    getType() {
        return this.bldr.getType();
    }
    setType(type) {
        this.bldr.setType(type);
        return this;
    }
    alias(alias) {
        this.bldr.alias(alias);
        return this;
    }
    description(description) {
        this.bldr.description(description);
        return this;
    }
}
export class EnumAst {
    values;
    bldr;
    constructor(tb, name, values = new Set()) {
        this.values = values;
        this.bldr = tb.getEnum(name);
    }
    type() {
        return this.bldr.field();
    }
}
export class EnumViewer extends EnumAst {
    constructor(tb, name, values = new Set()) {
        super(tb, name, values);
    }
    listValues() {
        return Array.from(this.values).map((name) => [name, new EnumValueViewer()]);
    }
    value(name) {
        if (!this.values.has(name)) {
            throw new Error(`Value ${name} not found.`);
        }
        return new EnumValueViewer();
    }
}
export class EnumValueViewer {
    constructor() { }
}
export class EnumBuilder extends EnumAst {
    constructor(tb, name, values = new Set()) {
        super(tb, name, values);
    }
    addValue(name) {
        if (this.values.has(name)) {
            throw new Error(`Value ${name} already exists.`);
        }
        this.values.add(name);
        return this.bldr.value(name);
    }
    listValues() {
        return Array.from(this.values).map((name) => [name, this.bldr.value(name)]);
    }
    value(name) {
        return this.bldr.value(name);
    }
}
