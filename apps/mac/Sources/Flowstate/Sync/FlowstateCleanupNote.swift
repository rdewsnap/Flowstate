import SwiftUI

/// Shown in the Cleanup tab in place of the key and model fields when the
/// Flowstate server is the cloud provider.
struct FlowstateCleanupNote: View {
    @ObservedObject private var account = AccountController.shared
    @ObservedObject private var prefs = Preferences.shared

    var body: some View {
        if account.isSignedIn {
            LabeledContent("Account", value: account.email ?? "Signed in")
            Text("Cleanup and meeting summaries run on \(account.usesCloud ? "Flowstate Cloud" : prefs.serverURL), which picks the model. Only text is sent. Past your plan's monthly allowance, dictations fall back to the filter on this Mac.")
                .font(.callout).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if let usage = account.me?.usage, usage.requestsLimit == 0 {
                Text("Your \(usage.planName) plan doesn't include hosted cleanup.")
                    .font(.callout).foregroundStyle(.orange)
            }
        } else {
            HStack {
                Text("Sign in to use your Flowstate account for cleanup.").foregroundStyle(.orange)
                Spacer()
                Button("Open Account") { SettingsRouter.shared.tab = .account }
            }
        }
    }
}
