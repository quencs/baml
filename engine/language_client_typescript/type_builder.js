"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.EnumValueBuilder = exports.EnumBuilder = exports.ClassBuilder = exports.TypeBuilder = void 0;
const native_1 = require("../native");
class TypeBuilder {
    tb;
    constructor({ runtime }) {
        this.tb = native_1.TypeBuilder.new(runtime);
    }
    _tb() {
        return this.tb;
    }
    reset() {
        this.tb.reset();
    }
    toString() {
        return this.tb.toString();
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
    null() {
        return this.tb.null();
    }
    map(keyType, valueType) {
        return this.tb.map(keyType, valueType);
    }
    union(types) {
        return this.tb.union(types);
    }
    addClass(name) {
        return new ClassBuilder(this.tb.addClass(name));
    }
    getClass(name) {
        return new ClassBuilder(this.tb.getClass(name));
    }
    addEnum(name) {
        return new EnumBuilder(this.tb.addEnum(name));
    }
    getEnum(name) {
        return new EnumBuilder(this.tb.getEnum(name));
    }
    addBaml(baml) {
        this.tb.addBaml(baml);
    }
}
exports.TypeBuilder = TypeBuilder;
class ClassBuilder {
    cb;
    constructor(cb) {
        this.cb = cb;
    }
    type() {
        return this.cb.field();
    }
    listProperties() {
        return this.cb
            .listProperties()
            .map(([name, property]) => [
            name,
            new ClassPropertyBuilder(property),
        ]);
    }
    reset() {
        return this.cb.reset();
    }
    getProperty(name) {
        return new ClassPropertyBuilder(this.cb.getProperty(name));
    }
    /**
     * addProperty:
     *  - only allowed if Class marked with @@dynamic
     */
    addProperty(name, fieldType) {
        let cpb = this.cb.addProperty(name, fieldType);
        return new ClassPropertyBuilder(cpb);
    }
    /**
     * removeProperty:
     *  - only allowed if Class marked with @@dynamic
     */
    removeProperty(name) {
        this.cb.removeProperty(name);
    }
    /**
     * setAlias:
     *  - only allowed if Class marked with @@dynamic
     */
    setAlias(alias) {
        this.cb.setAlias(alias);
        return this;
    }
    /**
     * setDescription:
     *  - only allowed if Class marked with @@dynamic
     */
    setDescription(description) {
        this.cb.setDescription(description);
        return this;
    }
    alias() {
        return this.cb.alias();
    }
    description() {
        return this.cb.description();
    }
    source() {
        return this.cb.source();
    }
}
exports.ClassBuilder = ClassBuilder;
class EnumBuilder {
    eb;
    constructor(eb) {
        this.eb = eb;
    }
    type() {
        return this.eb.field();
    }
    listValues() {
        return this.eb
            .listValues()
            .map(([name, value]) => [
            name,
            new EnumValueBuilder(value),
        ]);
    }
    /**
     * addValue:
     *  - only allowed if Enum marked with @@dynamic
     */
    addValue(name) {
        let evb = this.eb.addValue(name);
        return new EnumValueBuilder(evb);
    }
    getValue(name) {
        let evb = this.eb.getValue(name);
        return new EnumValueBuilder(evb);
    }
    /**
     * removeValue:
     *  - only allowed if Enum marked with @@dynamic
     */
    removeValue(name) {
        this.eb.removeValue(name);
        return this;
    }
    /**
     * setAlias:
     *  - only allowed if Enum marked with @@dynamic
     */
    setAlias(alias) {
        this.eb.setAlias(alias);
        return this;
    }
    /**
     * setDescription:
     *  - only allowed if Enum marked with @@dynamic
     */
    setDescription(description) {
        this.eb.setDescription(description);
        return this;
    }
    alias() {
        return this.eb.alias();
    }
    description() {
        return this.eb.description();
    }
}
exports.EnumBuilder = EnumBuilder;
class ClassPropertyBuilder {
    cpb;
    constructor(cpb) {
        this.cpb = cpb;
    }
    type() {
        return this.cpb.getType();
    }
    setType(fieldType) {
        this.cpb.setType(fieldType);
        return this;
    }
    setAlias(alias) {
        this.cpb.setAlias(alias);
        return this;
    }
    setDescription(description) {
        this.cpb.setDescription(description);
        return this;
    }
    alias() {
        return this.cpb.alias();
    }
    description() {
        return this.cpb.description();
    }
    source() {
        return this.cpb.source();
    }
}
class EnumValueBuilder {
    evb;
    constructor(evb) {
        this.evb = evb;
    }
    setAlias(alias) {
        this.evb.setAlias(alias);
        return this;
    }
    setDescription(description) {
        this.evb.setDescription(description);
        return this;
    }
    alias() {
        return this.evb.alias();
    }
    description() {
        return this.evb.description();
    }
    source() {
        return this.evb.source();
    }
}
exports.EnumValueBuilder = EnumValueBuilder;
