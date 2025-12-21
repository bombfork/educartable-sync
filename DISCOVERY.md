# Educartable Website Discovery

## Authentication System

### OpenID Connect with Keycloak

**Provider**: Keycloak Identity and Access Management
**Domain**: https://accounts.edumoov.com/
**Realm**: edumoov
**Protocol**: OpenID Connect (OIDC)

**Authentication Flow**:
- User visits https://app.educartable.com
- Redirected to Keycloak login page: `https://accounts.edumoov.com/auth/realms/edumoov/protocol/openid-connect/auth` with URL parameters

### OIDC Flow Details

Standard OpenID Connect flow typically involves:
1. **Authorization Request**: Browser redirects to Keycloak with parameters (client_id, redirect_uri, response_type, scope, state)
2. **User Authentication**: User enters credentials on Keycloak page
3. **Authorization Response**: Keycloak redirects back to app with authorization code
4. **Token Exchange**: App exchanges authorization code for tokens (access_token, refresh_token, id_token)
5. **API Requests**: App uses access_token to authenticate API calls

### Captured OIDC Parameters

**Authorization Endpoint**: `https://accounts.edumoov.com/auth/realms/edumoov/protocol/openid-connect/auth`

**Parameters**:
- ✅ **client_id**: `educlass`
- ✅ **redirect_uri**: `https://app.educartable.com/`
- ✅ **response_type**: `code` (Authorization Code Flow)
- ✅ **scope**: `openid`
- ✅ **state**: Random CSRF protection token (generated per request)
- ✅ **code_challenge**: PKCE challenge (generated per request)
- ✅ **code_challenge_method**: `S256` (SHA-256 hashing)
- ✅ **response_mode**: `query` (authorization code returned in URL query parameter)

**Authentication Flow Type**: OAuth2 Authorization Code Flow with PKCE (Proof Key for Code Exchange)

### PKCE Flow Explanation

PKCE (RFC 7636) adds security to the authorization code flow:
1. Generate random `code_verifier` (43-128 characters)
2. Create `code_challenge` = BASE64URL(SHA256(code_verifier))
3. Send `code_challenge` and `code_challenge_method=S256` with auth request
4. After receiving authorization code, exchange it with original `code_verifier`

### Post-Login Redirect

**Redirect URL Format**:
```
https://app.educartable.com/home?state=<state>&session_state=<session>&iss=https%3A%2F%2Faccounts.edumoov.com%2Fauth%2Frealms%2Fedumoov&code=<authorization_code>
```

**Parameters Received**:
- ✅ **state**: CSRF token (must match the one sent in auth request)
- ✅ **session_state**: Keycloak session identifier
- ✅ **iss**: Issuer URL (the Keycloak realm URL, URL-encoded)
- ✅ **code**: Authorization code to exchange for tokens

**Redirect Target**: `/home` path (not root `/`)

### Token Exchange

**OIDC Client Library**: The application uses an OIDC client library (likely `oidc-client-js` or `oidc-client-ts`)

**Console Log Observed**:
```
[api/keycloak/oidc] UserManager.signinRedirectCallback: successful, signed in sub: b7077257-dfe3-4776-a49d-2d0e5e8410c0
```

**Token Storage**: Tokens are handled by the OIDC client and stored in browser storage (localStorage/sessionStorage)

**Token Exchange**:
- Happens automatically via JavaScript (OIDC client library)
- Token endpoint (confirmed): `https://accounts.edumoov.com/auth/realms/edumoov/protocol/openid-connect/token`

**Token Exchange Request** (POST):
```
Content-Type: application/x-www-form-urlencoded

Parameters:
- client_id: "educlasse"
- code: "<authorization_code from redirect>"
- redirect_uri: "https://app.educartable.com/activities"
- code_verifier: "<PKCE verifier>"
- grant_type: "authorization_code"
```

**Token Exchange Response** (JSON):
```json
{
  "access_token": "<JWT token>",
  "expires_in": 3600,
  "refresh_expires_in": 10800,
  "refresh_token": "<JWT refresh token>",
  "token_type": "Bearer",
  "id_token": "<JWT ID token>",
  "not-before-policy": 1527022222,
  "session_state": "<session-uuid>",
  "scope": "openid"
}
```

**Token Lifetimes**:
- Access Token: 3600 seconds (1 hour)
- Refresh Token: 10800 seconds (3 hours)

**Note**: `redirect_uri` points to `/activities` not root `/`

### Token Storage Details

**Storage Location**: localStorage (sessionStorage is empty)

**Keys Found**:
1. `loglevel`: `INFO`
2. `oidc.user:https://accounts.edumoov.com/auth/realms/edumoov:educlasse`: User and token data

**Complete Token Structure** (stored in localStorage):
```json
{
  "id_token": "<JWT token>",
  "session_state": "<session-uuid>",
  "access_token": "<JWT access token>",
  "refresh_token": "<JWT refresh token>",
  "token_type": "Bearer",
  "scope": "openid",
  "profile": {
    "exp": 1766315826,
    "iat": 1766312226,
    "auth_time": 1766312225,
    "jti": "<uuid>",
    "iss": "https://accounts.edumoov.com/auth/realms/edumoov",
    "aud": "educlasse",
    "sub": "<user-id>",
    "typ": "ID",
    "azp": "educlasse",
    "sid": "<session-id>",
    "at_hash": "<hash>",
    "name": "<full-name>",
    "preferred_username": "<email>",
    "given_name": "<first-name>",
    "family_name": "<last-name>",
    "email": "<email>"
  },
  "expires_at": 1766315826
}
```

**Token Details**:
- ✅ **id_token**: JWT ID token (contains user profile info)
- ✅ **access_token**: JWT access token (used for API authorization)
- ✅ **refresh_token**: JWT refresh token (used to obtain new access tokens)
- ✅ **token_type**: `Bearer`
- ✅ **expires_at**: Unix timestamp (1 hour = 3600 seconds from issuance)

**Token Refresh**: Refresh token available for obtaining new access tokens without re-authentication

**Note**: The client_id is `educlasse` (with an 'e' at the end), not `educlass`

### Authorization Header Format

**Header Format**: `Authorization: <access_token>` (JWT token directly, **no "Bearer" prefix**)

**Example**: `Authorization: eyJhbGciOiJSUzI1NiIsIn...`

⚠️ **Note**: This is non-standard. Most OAuth2/OIDC implementations use `Authorization: Bearer <token>`, but Educartable uses the token directly without the "Bearer" prefix.

## API Structure

### API Base URLs

1. **Main API**: `https://app.educartable.com/api/1.0/`
2. **RPC Endpoints**: `https://app.educartable.com/rpc/`
3. **Media/Images**: `https://www.edumoov.com/api/1.0/` (different domain!)

### Key Endpoints

**User & Profile**:
- `GET /api/1.0/educore/users/me?light=1` - Current user info
- `GET /api/1.0/educore/parent/{parent_id}/pupils` - Children/pupils info
- `GET /api/1.0/educore/parent/{parent_id}/settings?app=EducartableFamily` - User settings
- `GET /api/1.0/educore/parent/{parent_id}/notifications?app=Educartable` - Notifications

**Messages/Activities** (contains articles with pictures):
- `GET /api/1.0/educartable/parent/{parent_id}/messages?type=activity&sort=date&direction=desc` - Activities (articles with pictures)
- `GET /api/1.0/educartable/parent/{parent_id}/messages?type=lesson&sort=date&direction=asc&start=2025-12-21&limit=30` - Lessons
- `GET /api/1.0/educartable/parent/{parent_id}/messages?type=info,event,alert,meeting,advert&sort=visibility&direction=desc` - Other messages

**Events**:
- `POST /rpc/cartable.events.nextRegisteredEvents` - Upcoming events
  - Body: `{"params":{"start":"2025-12-21","stop":"2026-03-21"},"payload":{}}`

**Media/Images**:
- `GET https://www.edumoov.com/api/1.0/educore/medias/{media_uuid}/thumbnail?width=300&mode=cover` - Thumbnail images

**Other**:
- `GET /api/1.0/livret/parent/{parent_id}/notebooks` - Notebooks
- `GET /api/1.0/livret/parent/{parent_id}/attestations_validations` - Attestations

### Parent ID
The API uses a `parent_id` parameter in URLs (e.g., `1954900`). This ID is likely available from the `/users/me` endpoint or from the JWT token profile.

### Messages/Activities Response Structure

**Endpoint**: `GET /api/1.0/educartable/parent/{parent_id}/messages?type=activity&sort=date&direction=desc`

**Response Structure**:
```json
{
  "success": true,
  "data": [
    {
      "id": "<activity-uuid>",
      "title": "Activity title",
      "body": "HTML content",
      "type": "activity",
      "date": "2025-12-17T13:39:08+00:00",
      "visibility": "2025-12-17T13:57:03+00:00",
      "user": {
        "id": 752054,
        "name": "LASTNAME",
        "firstname": "FIRSTNAME"
      },
      "medias": [
        {
          "id": "76790a51-506b-4b79-9d97-c44fc1bd0e92",
          "name": "001.A",
          "description": "",
          "user_id": 752054,
          "size": 427394,
          "extension": ".JPG",
          "type": "image/jpeg"
        }
      ],
      "pupils": [6261593]
    }
  ],
  "pagination": {
    "page_count": 4,
    "current_page": 1,
    "has_next_page": true,
    "has_prev_page": false,
    "count": 40,
    "limit": 10
  }
}
```

**Key Findings**:
- ✅ Activities are paginated (10 per page)
- ✅ Each activity has a `medias` array
- ✅ Each media has a UUID `id` used to construct download URLs
- ✅ Media includes images (JPEG, PNG) and videos (MOV)
- ✅ File size and extension are provided
- ✅ `pupils` array shows which children are associated with the activity

### Image Download Flow

**Two-step download process**:

1. **Step 1 - Request signed URL** (requires authentication):
   ```
   GET https://www.edumoov.com/api/1.0/educore/medias/{media_uuid}/file?cache=1&filename={filename}
   Headers: Authorization: {access_token}
   Response: 302 Redirect
   ```

2. **Step 2 - Download from CDN** (temporary signed URL):
   ```
   GET https://filerz.edumoov.com/edumoov-2/{media_uuid}?temp_url_sig={signature}&temp_url_expires={timestamp}&filename={filename}
   Response: Image file (direct download)
   ```

**Key Details**:
- ✅ Initial `/file` endpoint returns 302 redirect to CDN
- ✅ CDN URL is on `filerz.edumoov.com` (separate domain)
- ✅ URL includes temporary signature (`temp_url_sig`) and expiration (`temp_url_expires`)
- ✅ Signed URLs are time-limited (expires at Unix timestamp)
- ✅ Filename is preserved in query parameter

**Download Strategy**:
1. Call `/file` endpoint with authentication to get signed URL
2. Follow 302 redirect to CDN
3. Download file from signed CDN URL

### CDN Authentication

**✅ CONFIRMED**: Signed CDN URLs do NOT require authentication headers.

The temporary signed URLs returned from the `/file` endpoint are self-contained and can be accessed without any authentication. The signature and expiration timestamp provide the security.

**Implication**: Once we obtain the signed CDN URL, we can download the file with a simple HTTP GET request without passing any authorization headers.

## Website Architecture

- Single Page Application (SPA) / Progressive Web App (PWA)
- JavaScript-heavy frontend
- RESTful API backend
- CDN for media storage (separate domain)
- OAuth2/OIDC authentication via Keycloak

## Discovery Summary

### ✅ Complete Authentication Flow
1. User initiates OAuth2 authorization code flow with PKCE
2. Browser redirects to Keycloak at `accounts.edumoov.com`
3. User enters credentials
4. Keycloak redirects back with authorization code
5. JavaScript exchanges code for tokens (access_token, refresh_token, id_token)
6. Tokens stored in localStorage
7. API calls include `Authorization: {access_token}` header (no "Bearer" prefix)

### ✅ Complete Picture Download Flow
1. Authenticate and obtain access_token
2. Get user info: `GET /api/1.0/educore/users/me`
3. Get parent_id from user info
4. Fetch activities with pictures: `GET /api/1.0/educartable/parent/{parent_id}/messages?type=activity`
5. Parse pagination to get all activities
6. Extract media UUIDs from `medias` array in each activity
7. For each media UUID, request signed URL: `GET https://www.edumoov.com/api/1.0/educore/medias/{uuid}/file?cache=1&filename={name}`
8. Follow 302 redirect to CDN (or extract Location header)
9. Download file from CDN signed URL (no auth required)

### ✅ Key Technical Details
- **client_id**: `educlasse`
- **Token lifetime**: Access token = 1 hour, Refresh token = 3 hours
- **API base**: `https://app.educartable.com/api/1.0/`
- **Media API**: `https://www.edumoov.com/api/1.0/`
- **CDN**: `https://filerz.edumoov.com/`
- **Authorization header**: Direct token (no "Bearer" prefix)
- **Pagination**: 10 items per page
- **Media types**: Images (JPEG, PNG) and Videos (MOV)

### Ready for Implementation
All required information has been gathered. We can now proceed with designing and implementing the downloader application.
