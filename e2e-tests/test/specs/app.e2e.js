import { expect } from '@wdio/globals'

describe('Educartable Sync Application', () => {
  it('should launch the application', async () => {
    // Wait for the app to be ready
    await browser.pause(1000)

    // Verify the browser object is available
    expect(browser).toBeDefined()
  })

  it('should display the main heading', async () => {
    // Find the main heading
    const heading = await $('h1')
    await heading.waitForExist({ timeout: 5000 })

    // Get the text content
    const text = await heading.getText()

    // Verify it contains "Educartable Sync"
    expect(text).toContain('Educartable Sync')
  })

  it('should display configuration section', async () => {
    // Find the configuration section
    const configSection = await $('#config-section')
    await configSection.waitForExist({ timeout: 5000 })

    // Verify it's displayed
    const isDisplayed = await configSection.isDisplayed()
    expect(isDisplayed).toBe(true)
  })

  it('should display authentication status', async () => {
    // Find the auth status element
    const authStatus = await $('#auth-status')
    await authStatus.waitForExist({ timeout: 5000 })

    // Get the text content
    const text = await authStatus.getText()

    // Verify it shows disconnected status
    expect(text).toContain('Non connecté')
  })

  it('should display login button', async () => {
    // Find the login button
    const loginBtn = await $('#login-btn')
    await loginBtn.waitForExist({ timeout: 5000 })

    // Verify it's enabled and displayed
    const isDisplayed = await loginBtn.isDisplayed()
    const isEnabled = await loginBtn.isEnabled()

    expect(isDisplayed).toBe(true)
    expect(isEnabled).toBe(true)
  })

  it('should display sync button in disabled state', async () => {
    // Find the sync button
    const syncBtn = await $('#sync-btn')
    await syncBtn.waitForExist({ timeout: 5000 })

    // Verify it's displayed but disabled
    const isDisplayed = await syncBtn.isDisplayed()
    const isEnabled = await syncBtn.isEnabled()

    expect(isDisplayed).toBe(true)
    expect(isEnabled).toBe(false)
  })

  it('should display browse button', async () => {
    // Find the browse button
    const browseBtn = await $('#browse-btn')
    await browseBtn.waitForExist({ timeout: 5000 })

    // Verify it's displayed and enabled
    const isDisplayed = await browseBtn.isDisplayed()
    const isEnabled = await browseBtn.isEnabled()

    expect(isDisplayed).toBe(true)
    expect(isEnabled).toBe(true)
  })
})
