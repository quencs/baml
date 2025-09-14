import pytest
from hamcrest import assert_that, equal_to
from baml_py import Image, Audio, Pdf
from baml_client import b


@pytest.mark.asyncio
async def test_expose_request_openai_responses_multimodal():
    test_image = Image.from_url(
        "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg"
    )
    request = await b.request.TestOpenAIResponsesImageInput(test_image)

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "o1-mini",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {"type": "input_text", "text": "what is in this content?"},
                            {
                                "type": "input_image",
                                "image_url": "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg",
                            },
                        ],
                    }
                ],
            }
        ),
    )


@pytest.mark.asyncio
async def test_expose_request_openai_responses_audio():
    test_audio_data = "UklGRnoGAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQoGAACBhYqFbF1fdJivrJBhNjVgodDbq2EcBj+a2/LDciUFLIHO8tiJNwgZaLvt559NEAxQp+PwtmMcBjiR1/LMeSwFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoFJHfH8N2QQAoUYrTp66hVFApGn+DyvmEaBC2Bye/OcyoF"
    test_audio = Audio.from_base64("audio/wav", test_audio_data)
    request = await b.request.TestOpenAIResponsesImageInput(test_audio)

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "o1-mini",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {"type": "input_text", "text": "what is in this content?"},
                            {
                                "type": "input_audio",
                                "input_audio": {
                                    "data": test_audio_data,
                                    "format": "wav",
                                },
                            },
                        ],
                    }
                ],
            }
        ),
    )


@pytest.mark.asyncio
async def test_expose_request_openai_responses_pdf_base64():
    # Test that base64 PDFs are sent as filename + file_data using a real file
    import base64
    from pathlib import Path

    pdf_path = Path(__file__).resolve().parents[3] / "baml_src" / "dummy.pdf"
    with open(pdf_path, "rb") as f:
        test_pdf_b64 = base64.b64encode(f.read()).decode("ascii")

    test_pdf = Pdf.from_base64(test_pdf_b64)
    request = await b.request.TestOpenAIResponsesImageInput(test_pdf)

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "o1-mini",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {"type": "input_text", "text": "what is in this content?"},
                            {
                                "type": "input_file",
                                "filename": "document.pdf",
                                "file_data": test_pdf_b64,
                            },
                        ],
                    }
                ],
            }
        ),
    )


@pytest.mark.asyncio
async def test_openai_responses_basic():
    request = await b.request.TestOpenAIResponses("lorem ipsum")

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "gpt-4.1",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "Write a short haiku about lorem ipsum. Make it simple and beautiful.",
                            },
                        ],
                    },
                ],
            }
        ),
    )


@pytest.mark.asyncio
async def test_openai_responses_explicit():
    request = await b.request.TestOpenAIResponsesExplicit("lorem ipsum")

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "gpt-4.1",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "Create a brief poem about lorem ipsum. Keep it under 50 words.",
                            },
                        ],
                    },
                ],
            }
        ),
    )


@pytest.mark.asyncio
async def test_openai_responses_custom_url():
    request = await b.request.TestOpenAIResponsesCustomURL("lorem ipsum")

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "gpt-4.1",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "Tell me an interesting fact about lorem ipsum.",
                            },
                        ],
                    },
                ],
            }
        ),
    )


@pytest.mark.asyncio
async def test_openai_responses_conversation():
    request = await b.request.TestOpenAIResponsesConversation("lorem ipsum")

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "gpt-4.1",
                "input": [
                    {
                        "role": "system",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "You are a helpful assistant that provides concise answers.",
                            },
                        ],
                    },
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "What is lorem ipsum?",
                            },
                        ],
                    },
                    {
                        "role": "assistant",
                        "content": [
                            {
                                "type": "output_text",
                                "text": "lorem ipsum is a fascinating subject. Let me explain briefly.",
                            },
                        ],
                    },
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "Can you give me a simple example?",
                            },
                        ],
                    },
                ],
            }
        ),
    )


@pytest.mark.asyncio
async def test_openai_responses_different_model():
    request = await b.request.TestOpenAIResponsesDifferentModel("lorem ipsum")

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "gpt-4",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "Explain lorem ipsum in one sentence.",
                            },
                        ],
                    },
                ],
            }
        ),
    )


@pytest.mark.asyncio
async def test_expose_request_openai_responses_pdf_url():
    # Test that PDF URLs are preserved as URLs (OpenAI Responses API supports file_url with URLs)
    test_pdf = Pdf.from_url(
        "https://www.usenix.org/system/files/conference/nsdi13/nsdi13-final85.pdf"
    )
    request = await b.request.TestOpenAIResponsesImageInput(test_pdf)

    assert_that(
        request.body.json(),
        equal_to(
            {
                "model": "o1-mini",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {"type": "input_text", "text": "what is in this content?"},
                            {
                                "type": "input_file",
                                "file_url": "https://www.usenix.org/system/files/conference/nsdi13/nsdi13-final85.pdf",
                            },
                        ],
                    }
                ],
            }
        ),
    )
