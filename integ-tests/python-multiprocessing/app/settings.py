"""Minimal Django settings for BAML observability test."""

SECRET_KEY = "test-secret-key-not-for-production"

DEBUG = True

INSTALLED_APPS = [
    "django_rq",
]

# Redis Queue configuration
RQ_QUEUES = {
    "default": {
        "HOST": "localhost",
        "PORT": 6379,
        "DB": 0,
    },
    "async": {
        "HOST": "localhost",
        "PORT": 6379,
        "DB": 0,
        "ASYNC": True,  # Enable async job support
    },
}

# Required for Django
USE_TZ = True
