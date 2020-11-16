POETRY ?= poetry

all: lint-check build test

build:
	@$(POETRY) run maturin develop

.PHONY: install-poetry
install-poetry:
	@curl -sSL https://raw.githubusercontent.com/sdispater/poetry/master/get-poetry.py | python

.PHONY: install-deps
install-deps:
	@$(POETRY) install -vv

.PHONY: install
install: install-poetry install-deps

.PHONY: lint-black
lint-black:
	@echo "\033[92m< linting using black...\033[0m"
	@$(POETRY) run black .
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-flake8
lint-flake8:
	@echo "\033[92m< linting using flake8...\033[0m"
	@$(POETRY) run flake8 capara tests
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-isort
lint-isort:
	@echo "\033[92m< linting using isort...\033[0m"
	@$(POETRY) run isort --recursive .
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-mypy
lint-mypy:
	@echo "\033[92m< linting using mypy...\033[0m"
	@$(POETRY) run mypy --ignore-missing-imports --follow-imports=silent capara tests
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-cargo-fmt
lint-cargo-fmt:
	@echo "\033[92m< linting using cargo-fmt...\033[0m"
	cargo fmt
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint
lint: lint-cargo-fmt lint-black lint-flake8 lint-isort lint-mypy

.PHONY: lint-check-black
lint-check-black:
	@echo "\033[92m< linting using black...\033[0m"
	@$(POETRY) run black --check --diff .
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-check-flake8
lint-check-flake8:
	@echo "\033[92m< linting using flake8...\033[0m"
	@$(POETRY) run flake8 capara tests
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-check-isort
lint-check-isort:
	@echo "\033[92m< linting using isort...\033[0m"
	@$(POETRY) run isort --check-only --diff --recursive .
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-check-mypy
lint-check-mypy:
	@echo "\033[92m< linting using mypy...\033[0m"
	@$(POETRY) run mypy --ignore-missing-imports --follow-imports=silent capara tests
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-check-clippy
lint-check-clippy:
	@echo "\033[92m< linting using clippy...\033[0m"
	cargo clippy
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-check-rustfmt
lint-check-rustfmt:
	@echo "\033[92m< linting using cargo fmt...\033[0m"
	cargo fmt --all -- --check
	@echo "\033[92m> done\033[0m"
	@echo

.PHONY: lint-check
lint-check: lint-check-clippy lint-check-rustfmt lint-check-black lint-check-flake8 lint-check-isort lint-check-mypy

.PHONY: test
test:
	@$(POETRY) run pytest


.PHONY: publish
publish:
	twine upload --non-interactive --skip-existing target/wheels/*.whl target/wheels/*.tar.gz