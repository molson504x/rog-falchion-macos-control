# ROG Falchion macOS Control

An early-stage project to make it possible to configure the ASUS ROG Falchion keyboard from macOS. ASUS provides Armoury Crate for configuring the keyboard, but its download requirements currently list Windows 10 (version 1903 or later) or Windows 11, not macOS.

**Project status:** this repository does not yet contain a runnable application. The capabilities below describe development goals, not features that are already implemented.

## About

The goal is to build a user-friendly application for configuring the original ROG Falchion without requiring a Windows PC. The project is starting with macOS and aims to avoid a design that is tied to Apple-only technologies, so support for other platforms may be possible later.

The ROG Falchion is a compact 65% wireless mechanical keyboard. It supports wired USB and 2.4 GHz wireless connections, per-key RGB lighting, a touch panel, and onboard profiles. Configuration is currently provided through ASUS Armoury Crate.

## Installation

There is no application release or installation procedure yet. Once the application and its build tooling are available, installation instructions will be added here.

For ASUS's current Windows software and device downloads, see the [ROG Falchion support and downloads page](https://rog.asus.com/us/keyboards/keyboards/compact/rog-falchion-model/helpdesk_download/). ASUS currently lists Armoury Crate for Windows 10 (version 1903 or later) and Windows 11.

## Usage and planned capabilities

The project is intended to explore configuration options such as:

- Customizing keyboard lighting and RGB effects.
- Recording, assigning, and clearing macros.
- Remapping keys and managing keyboard profiles.
- Configuring the side touch panel.
- Investigating function-key behavior, including whether swapping Fn and Windows-key behavior is supported.

These are goals for development, not a promise that every feature is supported by the keyboard or will be implemented. ASUS's specifications list six onboard profiles and programmable keys except Fn and Shift; available controls may depend on the keyboard firmware and connection mode.

## Development (local debug)

The application source code and development toolchain have not been added yet, so there are no build or debug commands at this stage. Development setup instructions will be documented here when the project has a runnable implementation.

## Running unit tests

No test suite or test command exists yet. The project aims to add unit tests alongside implementation, with positive and negative cases and a target of at least 90% code coverage.

## Repository structure

This repository is currently in its initial documentation and planning stage. Application source code, distribution artifacts, and project-specific build or test configuration have not been added yet.

## Product references

- [ASUS ROG Falchion product page](https://rog.asus.com/us/keyboards/keyboards/compact/rog-falchion-model/)
- [ASUS ROG Falchion technical specifications](https://rog.asus.com/us/keyboards/keyboards/compact/rog-falchion-model/spec/)
- [ASUS ROG Falchion support and downloads](https://rog.asus.com/us/keyboards/keyboards/compact/rog-falchion-model/helpdesk_download/)
