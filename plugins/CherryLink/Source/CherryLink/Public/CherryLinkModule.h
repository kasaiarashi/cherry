// Copyright Krishna Teja Mekala. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class FCherryLinkServer;
class FBlueprintService;
class FLogService;
class FPlayService;
class FBuildService;
class FExtender;

DECLARE_LOG_CATEGORY_EXTERN(LogCherryLink, Log, All);

/**
 * CherryLink Module - Provides integration between Unreal Editor and Cherry IDE
 *
 * This module manages:
 * - TCP server for communication with Cherry's helper binary
 * - Services for Blueprint, Logging, Play control, and Build integration
 */
class FCherryLinkModule : public IModuleInterface
{
public:
	virtual void StartupModule() override;
	virtual void ShutdownModule() override;

	static FCherryLinkModule& Get();
	static bool IsAvailable();

	TSharedPtr<FCherryLinkServer> GetServer() const { return Server; }
	TSharedPtr<FBlueprintService> GetBlueprintService() const { return BlueprintService; }
	TSharedPtr<FLogService> GetLogService() const { return LogService; }
	TSharedPtr<FPlayService> GetPlayService() const { return PlayService; }
	TSharedPtr<FBuildService> GetBuildService() const { return BuildService; }

	int32 GetServerPort() const { return ServerPort; }
	void SetServerPort(int32 Port);
	bool IsClientConnected() const;

private:
	void InitializeServices();
	void ShutdownServices();
	void RegisterMenus();
	void UnregisterMenus();
	void WritePortFile();
	void DeletePortFile();

private:
	TSharedPtr<FCherryLinkServer> Server;
	TSharedPtr<FBlueprintService> BlueprintService;
	TSharedPtr<FLogService> LogService;
	TSharedPtr<FPlayService> PlayService;
	TSharedPtr<FBuildService> BuildService;

	int32 ServerPort = 21567;
	TSharedPtr<FExtender> MenuExtender;
};
