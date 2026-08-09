# Tactica

Tactica separates access to the application from the people represented in
organisation rosters.

## Language

**Account**:
An application-wide security principal with a unique, canonical email address
whose ownership must be verified. An Account may have zero or more Identities
and may exist independently of any Person.
_Avoid_: User, Person, profile

**Person**:
A person represented in an organisation roster. A Person does not need an
Account.
_Avoid_: Account, user

**Identity**:
A means by which an Account authenticates, such as a password or an external
provider identity. Each Identity belongs to exactly one Account.
_Avoid_: Account, Person, login account

**Session**:
Authentication state through which a client acts as an Account. A Session
expires after sustained inactivity and cannot exceed its maximum lifetime.
_Avoid_: Identity, login
