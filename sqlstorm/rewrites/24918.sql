
WITH RankedPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        P.ViewCount,
        U.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY P.PostTypeId ORDER BY P.Score DESC, P.Id) AS Rank
    FROM 
        Posts P
        JOIN Users U ON P.OwnerUserId = U.Id
    WHERE 
        P.CreationDate >= '2024-10-01 12:34:56'::timestamp - INTERVAL '1 year'
        AND P.Score IS NOT NULL
),
TagStatistics AS (
    SELECT 
        T.Id AS TagId,
        T.TagName,
        COUNT(P.Id) AS PostCount,
        SUM(COALESCE(P.ViewCount, 0)) AS TotalViews
    FROM 
        Tags T
        LEFT JOIN Posts P ON P.Tags LIKE '%' || T.TagName || '%' 
    GROUP BY 
        T.Id, T.TagName
),
PostScores AS (
    SELECT 
        PH.PostId,
        MAX(PH.CreationDate) AS LastHistoryChange,
        SUM(CASE 
            WHEN PH.PostHistoryTypeId IN (10, 12) THEN 1 
            ELSE 0 
        END) AS CloseDeleteCount,
        SUM(CASE 
            WHEN PH.PostHistoryTypeId = 19 THEN 1 
            ELSE 0 
        END) AS ProtectedCount
    FROM 
        PostHistory PH
    GROUP BY 
        PH.PostId
),
UserSummaries AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COUNT(B.Id) AS BadgeCount,
        SUM(B.Class) AS TotalBadges,
        COALESCE(SUM(P.Score), 0) AS UserScore
    FROM 
        Users U
        LEFT JOIN Badges B ON U.Id = B.UserId
        LEFT JOIN Posts P ON U.Id = P.OwnerUserId
    GROUP BY 
        U.Id, U.DisplayName
),
CombinedStatistics AS (
    SELECT 
        RP.PostId,
        RP.Title,
        RP.OwnerDisplayName,
        RP.CreationDate,
        RP.Score,
        COALESCE(PS.LastHistoryChange, NULL) AS LastHistoryChange,
        COALESCE(PS.CloseDeleteCount, 0) AS CloseDeleteCount,
        COALESCE(PS.ProtectedCount, 0) AS ProtectedCount,
        TS.TagId,
        TS.TagName,
        TS.PostCount,
        TS.TotalViews,
        US.UserId,
        US.DisplayName AS UserDisplayName,
        US.BadgeCount,
        US.TotalBadges,
        US.UserScore
    FROM 
        RankedPosts RP
        LEFT JOIN PostScores PS ON RP.PostId = PS.PostId
        LEFT JOIN TagStatistics TS ON RP.PostId = TS.TagId
        LEFT JOIN UserSummaries US ON RP.OwnerDisplayName = US.DisplayName
    WHERE 
        RP.Rank <= 5
)

SELECT 
    CBS.PostId,
    CBS.Title,
    CBS.OwnerDisplayName,
    CBS.CreationDate,
    CBS.Score,
    CBS.LastHistoryChange,
    CBS.CloseDeleteCount,
    CBS.ProtectedCount,
    CBS.TagName,
    CBS.PostCount,
    CBS.TotalViews,
    CBS.UserDisplayName,
    CBS.BadgeCount,
    CBS.TotalBadges,
    CBS.UserScore
FROM 
    CombinedStatistics CBS
WHERE 
    CBS.Score > 10 
  AND CBS.CloseDeleteCount = 0
ORDER BY 
    CBS.Score DESC, CBS.CreationDate ASC
LIMIT 100;
