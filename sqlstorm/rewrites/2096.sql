WITH UserStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        U.Reputation,
        COUNT(DISTINCT P.Id) AS PostCount,
        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
        AVG(COALESCE(P.Score, 0)) AS AvgScore,
        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS ReputationRank
    FROM 
        Users U
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId
    GROUP BY 
        U.Id, U.DisplayName, U.Reputation
),
RecentPostVotes AS (
    SELECT 
        P.Id AS PostId,
        COUNT(V.Id) AS VoteCount,
        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
    FROM 
        Posts P
    LEFT JOIN 
        Votes V ON P.Id = V.PostId
    WHERE 
        P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '30 days'
    GROUP BY 
        P.Id
),
TopPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        COALESCE(RPV.VoteCount, 0) AS TotalVotes,
        COALESCE(RPV.UpVotes, 0) - COALESCE(RPV.DownVotes, 0) AS Score
    FROM 
        Posts P
    LEFT JOIN 
        RecentPostVotes RPV ON P.Id = RPV.PostId
    WHERE 
        P.PostTypeId = 1
    ORDER BY 
        Score DESC, P.Id
    LIMIT 10
)
SELECT 
    US.DisplayName,
    US.Reputation,
    US.PostCount,
    US.QuestionCount,
    US.AvgScore,
    TP.Title AS TopPostTitle,
    TP.TotalVotes,
    TP.Score,
    TP.PostId AS TopPostId,
    RPT.*  
FROM 
    UserStats US
INNER JOIN 
    TopPosts TP ON US.UserId = TP.PostId
LEFT JOIN 
    PostHistory RPT ON TP.PostId = RPT.PostId
WHERE 
    RPT.CreationDate = (
        SELECT MAX(CreationDate) 
        FROM PostHistory 
        WHERE PostId = TP.PostId
    )
ORDER BY 
    US.Reputation DESC, 
    TP.Score DESC;