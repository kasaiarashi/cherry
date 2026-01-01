// Copyright Krishna Teja Mekala. All Rights Reserved.

#include "CherryLinkModule.h"
#include "Server/CherryLinkServer.h"
#include "Services/BlueprintService.h"
#include "Services/LogService.h"
#include "Services/PlayService.h"
#include "Services/BuildService.h"
#include "SourceAccess/CherrySourceCodeAccessor.h"

#include "Misc/Paths.h"
#include "Misc/FileHelper.h"
#include "HAL/PlatformFilemanager.h"
#include "Interfaces/IProjectManager.h"
#include "Features/IModularFeatures.h"
#include "Modules/ModuleManager.h"

#define LOCTEXT_NAMESPACE "FCherryLinkModule"

DEFINE_LOG_CATEGORY(LogCherryLink);

void FCherryLinkModule::StartupModule()
{
	UE_LOG(LogCherryLink, Log, TEXT("CherryLink: Starting up..."));

	Server = MakeShared<FCherryLinkServer>();

	InitializeServices();

	if (Server->Start(ServerPort))
	{
		UE_LOG(LogCherryLink, Log, TEXT("CherryLink: Server started on port %d"), ServerPort);
		WritePortFile();
	}
	else
	{
		UE_LOG(LogCherryLink, Error, TEXT("CherryLink: Failed to start server on port %d"), ServerPort);
	}

	RegisterMenus();
	RegisterSourceCodeAccessor();
}

void FCherryLinkModule::RegisterSourceCodeAccessor()
{
	// Register Cherry as a source code editor
	CherrySourceCodeAccessor = MakeShared<FCherrySourceCodeAccessor>();
	CherrySourceCodeAccessor->Startup();

	// Register as a modular feature
	IModularFeatures::Get().RegisterModularFeature(TEXT("SourceCodeAccessor"), CherrySourceCodeAccessor.Get());

	UE_LOG(LogCherryLink, Log, TEXT("CherryLink: Registered Cherry as source code editor"));
}

void FCherryLinkModule::ShutdownModule()
{
	UE_LOG(LogCherryLink, Log, TEXT("CherryLink: Shutting down..."));

	UnregisterMenus();
	DeletePortFile();
	ShutdownServices();

	// Unregister source code accessor
	if (CherrySourceCodeAccessor.IsValid())
	{
		IModularFeatures::Get().UnregisterModularFeature(TEXT("SourceCodeAccessor"), CherrySourceCodeAccessor.Get());
		CherrySourceCodeAccessor->Shutdown();
		CherrySourceCodeAccessor.Reset();
	}

	if (Server.IsValid())
	{
		Server->Stop();
		Server.Reset();
	}
}

FCherryLinkModule& FCherryLinkModule::Get()
{
	return FModuleManager::GetModuleChecked<FCherryLinkModule>("CherryLink");
}

bool FCherryLinkModule::IsAvailable()
{
	return FModuleManager::Get().IsModuleLoaded("CherryLink");
}

void FCherryLinkModule::SetServerPort(int32 Port)
{
	if (Port != ServerPort && Port > 0 && Port < 65536)
	{
		ServerPort = Port;

		if (Server.IsValid() && Server->IsRunning())
		{
			Server->Stop();
			DeletePortFile();

			if (Server->Start(ServerPort))
			{
				WritePortFile();
			}
		}
	}
}

bool FCherryLinkModule::IsClientConnected() const
{
	return Server.IsValid() && Server->IsClientConnected();
}

void FCherryLinkModule::InitializeServices()
{
	BlueprintService = MakeShared<FBlueprintService>(Server);
	LogService = MakeShared<FLogService>(Server);
	PlayService = MakeShared<FPlayService>(Server);
	BuildService = MakeShared<FBuildService>(Server);

	BlueprintService->Initialize();
	LogService->Initialize();
	PlayService->Initialize();
	BuildService->Initialize();

	UE_LOG(LogCherryLink, Log, TEXT("CherryLink: Services initialized"));
}

void FCherryLinkModule::ShutdownServices()
{
	if (BuildService.IsValid())
	{
		BuildService->Shutdown();
		BuildService.Reset();
	}

	if (PlayService.IsValid())
	{
		PlayService->Shutdown();
		PlayService.Reset();
	}

	if (LogService.IsValid())
	{
		LogService->Shutdown();
		LogService.Reset();
	}

	if (BlueprintService.IsValid())
	{
		BlueprintService->Shutdown();
		BlueprintService.Reset();
	}

	UE_LOG(LogCherryLink, Log, TEXT("CherryLink: Services shut down"));
}

void FCherryLinkModule::RegisterMenus()
{
	// TODO: Register toolbar button and menu items
}

void FCherryLinkModule::UnregisterMenus()
{
	// TODO: Unregister toolbar button and menu items
}

void FCherryLinkModule::WritePortFile()
{
	FString PortFilePath = FPaths::ProjectIntermediateDir() / TEXT("CherryLink.port");
	FString PortString = FString::FromInt(ServerPort);

	if (FFileHelper::SaveStringToFile(PortString, *PortFilePath))
	{
		UE_LOG(LogCherryLink, Log, TEXT("CherryLink: Port file written to %s"), *PortFilePath);
	}
	else
	{
		UE_LOG(LogCherryLink, Warning, TEXT("CherryLink: Failed to write port file"));
	}
}

void FCherryLinkModule::DeletePortFile()
{
	FString PortFilePath = FPaths::ProjectIntermediateDir() / TEXT("CherryLink.port");

	if (FPlatformFileManager::Get().GetPlatformFile().FileExists(*PortFilePath))
	{
		FPlatformFileManager::Get().GetPlatformFile().DeleteFile(*PortFilePath);
		UE_LOG(LogCherryLink, Log, TEXT("CherryLink: Port file deleted"));
	}
}

#undef LOCTEXT_NAMESPACE

IMPLEMENT_MODULE(FCherryLinkModule, CherryLink)
