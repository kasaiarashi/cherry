// Copyright Krishna Teja Mekala. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"

class FCherryLinkServer;

/**
 * Service for controlling Play-In-Editor (PIE) sessions from Cherry.
 */
class CHERRYLINK_API FPlayService
{
public:
	FPlayService(TSharedPtr<FCherryLinkServer> InServer);
	~FPlayService();

	void Initialize();
	void Shutdown();

	/** PIE control methods */
	bool StartPIE(const FString& Mode = TEXT("PIE"), int32 PlayerCount = 1, bool bDedicatedServer = false);
	bool StopPIE();
	bool PausePIE();
	bool ResumePIE();
	bool TogglePause();

	/** State queries */
	bool IsPlaying() const;
	bool IsPaused() const;
	bool IsSimulating() const;
	FString GetCurrentState() const;

private:
	void OnPIEStarted(bool bSimulating);
	void OnPIEEnded(bool bSimulating);
	void BroadcastStateChange();

	/** Handle incoming requests */
	void HandleRequest(const FString& JsonMessage);

private:
	TWeakPtr<FCherryLinkServer> Server;

	FDelegateHandle OnStartedHandle;
	FDelegateHandle OnEndedHandle;
	FDelegateHandle OnMessageHandle;
};
