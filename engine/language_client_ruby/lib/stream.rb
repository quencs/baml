# typed: strict
require "sorbet-runtime"

module Baml
  class PendingResponse < T::Struct
    extend T::Sig
  end

  class BamlStream
    extend T::Sig
    extend T::Generic

    include Enumerable

    Elem = type_member(:out)
    FinalType = type_member(:out)

    sig { params(
      ffi_stream: Baml::Ffi::FunctionResultStream,
      ctx_manager: Baml::Ffi::RuntimeContextManager,
      partial_cast: T.proc.params(event: T.untyped).returns(Elem),
      final_cast: T.proc.params(event: T.untyped).returns(FinalType)
    ).void }
    def initialize(
      ffi_stream:,
      ctx_manager:,
      partial_cast:,
      final_cast:
    )
      @ffi_stream = ffi_stream
      @ctx_manager = ctx_manager
      @partial_cast = partial_cast
      @final_cast = final_cast
      @final_response = T.let(PendingResponse.new, T.any(PendingResponse, FinalType))
    end

    # Calls the given block once for each event in the stream, where event is a parsed
    # partial response. Returns `self` to enable chaining `.get_final_response`.
    #
    # Must be called with a block.
    #
    # @yieldparam [PartialType] event the parsed partial response
    # @return [BamlStream] self
    sig do
      override.params(block: T.proc.params(event: Elem).void)
        .returns(BamlStream[Elem, FinalType])
    end
    def each(&block)
      # Implementing this and include-ing Enumerable allows users to treat this as a Ruby
      # collection: https://ruby-doc.org/3.1.6/Enumerable.html#module-Enumerable-label-Usage
      case @final_response
      when PendingResponse
        @final_response = @ffi_stream.done(@ctx_manager) do |event|
          block.call(@partial_cast.call(event))
        end
      end
      self
    end

    # Gets the final response from the stream.
    #
    # @return [FinalType] the parsed final response
    sig {returns(FinalType)}
    def get_final_response
      case @final_response
      when PendingResponse
        @final_response = @ffi_stream.done(@ctx_manager)
      end

      @final_cast.call(@final_response)
    end
  end
end