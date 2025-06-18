# typed: strict
# This file should NOT be imported from baml.rb; we don't want
# to introduce a hard dependency on Sorbet for the baml gem.
require "ostruct"
require "json"
require "pp"
require "sorbet-runtime"

module Baml
  module Sorbet
    # Mixin that gives dynamically-typed “overflow” fields to any class
    module Struct
      extend T::Sig
      extend T::Helpers
      abstract!

      # --------------------------------------------------------------------
      # 1.  Declare & type the ivar once, so Sorbet knows it exists.
      #     Using `initialize` lets any including class call `super`.
      # --------------------------------------------------------------------
      sig { params(props: T::Hash[Symbol, T.untyped]).void }
      def initialize(props = {})
        super()                       # plays nicely with other mix-ins
        @props = T.let(
          props.transform_keys(&:to_sym),
          T::Hash[Symbol, T.untyped]
        )
      end

      # --------------------------------------------------------------------
      # 2.  Dynamic field access helpers
      # --------------------------------------------------------------------
      sig { params(symbol: Symbol).returns(T.untyped) }
      def method_missing(symbol)
        @props[symbol]
      end

      sig { params(key: T.untyped).returns(T.untyped) }
      def [](key)
        @props[key.to_sym]
      end

      # --------------------------------------------------------------------
      # 3.  Comparison & hashing
      # --------------------------------------------------------------------
      sig { params(other: T.untyped).returns(T::Boolean) }
      def eql?(other)
        T.unsafe(self).class == other.class &&
          @props.eql?(T.unsafe(other).instance_variable_get(:@props))
      end

      sig { returns(Integer) }
      def hash
        [T.unsafe(self).class, @props].hash
      end

      # --------------------------------------------------------------------
      # 4.  Pretty-printing helpers
      # --------------------------------------------------------------------
      sig { returns(String) }
      def inspect
        PP.pp(self, +"", 79)
      end

      sig { params(pp: PP).void }
      def pretty_print(pp)
        pp.object_group(self) do
          pp.breakable
          @props.each_with_index do |(k, v), idx|
            pp.text "#{k}="
            pp.pp v
            pp.comma_breakable unless idx == @props.size - 1
          end
        end
      end

      # --------------------------------------------------------------------
      # 5.  (De)serialisation helpers
      # --------------------------------------------------------------------
      sig { type_parameters(:V).params(block: T.nilable(T.proc.params(arg0: [Symbol, T.untyped]).returns(T.type_parameter(:V)))).returns(T.untyped) }
      def to_h(&block)
        block ? @props.map(&block).to_h : @props.dup
      end

      sig { params(state: T.untyped).returns(String) }
      def to_json(state = nil)
        state.nil? ? @props.to_json : @props.to_json(state)
      end
    end
  end

  # ------------------------------------------------------------------------
  # 6.  DynamicStruct: OpenStruct with a typed @table ivar so Sorbet is happy
  # ------------------------------------------------------------------------
  class DynamicStruct < OpenStruct
    extend T::Sig

    sig { params(hash: T.nilable(T::Hash[Symbol, T.untyped])).void }
    def initialize(hash = nil)
      super
      # OpenStruct creates @table, but Sorbet can’t see that.
      @table = T.let(@table, T::Hash[Symbol, T.untyped])
    end

    sig { params(state: T.untyped).returns(String) }
    def to_json(state = nil)
      state.nil? ? @table.to_json : @table.to_json(state)
    end
  end
end