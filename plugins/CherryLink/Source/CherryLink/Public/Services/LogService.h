// Copyright Krishna Teja Mekala. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Misc/OutputDevice.h"

class FCherryLinkServer;

/**
 * Service for capturing and streaming Unreal Engine logs to Cherry.
 * Implements FOutputDevice to hook into UE's logging system.
 */
class CHERRYLINK_API FLogService : public FOutputDevice
{
public:
	FLogService(TSharedPtr<FCherryLinkServer> InServer);
	virtual ~FLogService();

	void Initialize();
	void Shutdown();

	/** Configure log filtering */
	void SetCategoryFilter(const TArray<FName>& Categories);
	void SetMinVerbosity(ELogVerbosity::Type Verbosity);
	void ClearFilters();

	/** Check if logging is enabled */
	bool IsEnabled() const { return bIsEnabled; }
	void SetEnabled(bool bEnabled) { bIsEnabled = bEnabled; }

protected:
	// FOutputDevice interface
	virtual void Serialize(const TCHAR* Message, ELogVerbosity::Type Verbosity, const FName& Category) override;
	virtual void Serialize(const TCHAR* Message, ELogVerbosity::Type Verbosity, const FName& Category, const double Time) override;
	virtual bool CanBeUsedOnAnyThread() const override { return true; }
	virtual bool CanBeUsedOnMultipleThreads() const override { return true; }

private:
	FString VerbosityToString(ELogVerbosity::Type Verbosity) const;
	bool ShouldLogCategory(const FName& Category) const;
	void SendLogMessage(const TCHAR* Message, ELogVerbosity::Type Verbosity, const FName& Category);
	void HandleRequest(const FString& JsonMessage);

private:
	TWeakPtr<FCherryLinkServer> Server;

	TSet<FName> FilteredCategories;
	ELogVerbosity::Type MinVerbosity = ELogVerbosity::Log;
	bool bFilterByCategory = false;
	bool bIsEnabled = false;

	FCriticalSection LogLock;
	FDelegateHandle OnMessageHandle;
};
